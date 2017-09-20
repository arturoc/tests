#![cfg_attr(feature = "unstable", feature(test))]
#![cfg_attr(feature = "unstable", feature(get_type_id))]

#[cfg(test)]
extern crate rayon;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::mem;

use sync::*;
use storage::*;
pub use storage::{Read, Write, Storage, IntoIter, IntoIterMut, HierarchicalRead, HierarchicalWrite, HierarchicalStorage, IntoOrderedIter, IntoOrderedIterMut};
pub use entity::{Entity, Entities, EntitiesThreadLocal, EntityBuilder};
pub use component::{Component, ComponentSync, ComponentThreadLocal};
pub use dense_vec::DenseVec;
pub use forest::Forest;
pub use vec::VecStorage;

mod sync;
mod entity;
mod component;
mod storage;
mod dense_vec;
mod forest;
mod idtree;
mod vec;

#[cfg(test)]
mod tests;

#[cfg(feature="unstable")]
mod benches;
#[cfg(feature="unstable")]
mod parallel_benches;
#[cfg(feature="unstable")]
mod hierarchical_benches;

pub struct World{
    storages: HashMap<TypeId, Box<Any>>,
    storages_thread_local: HashMap<TypeId, Box<Any>>,

    next_guid: AtomicUsize,
    entities: Vec<Entity>, // Doesn't need lock cause never accesed mut from Entities?
    entities_index_per_mask: RwLock<HashMap<usize, Vec<usize>>>,
    ordered_entities_index_per_mask: RwLock<HashMap<TypeId, HashMap<usize, Vec<usize>>>>,
    reverse_components_mask_index: HashMap<usize, TypeId>,
    remove_components_mask_index: HashMap<usize, Box<Fn(&World, usize)>>,

    next_component_mask: AtomicUsize,
    components_mask_index: HashMap<TypeId, usize>,
}

impl World{
    pub fn new() -> World{
        World{
            storages: HashMap::new(),
            storages_thread_local: HashMap::new(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: AtomicUsize::new(1),
            entities: Vec::new(),
            components_mask_index: HashMap::new(),
            entities_index_per_mask: RwLock::new(HashMap::new()),
            ordered_entities_index_per_mask: RwLock::new(HashMap::new()),
            reverse_components_mask_index: HashMap::new(),
            remove_components_mask_index: HashMap::new(),
        }
    }

    pub fn register<C: ComponentSync>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RwLock::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask_mut = self.next_component_mask.get_mut();
        let next_mask = *next_mask_mut;
        *next_mask_mut *= 2;
        self.components_mask_index.insert(type_id, next_mask);
        self.reverse_components_mask_index.insert(next_mask, type_id);
        self.remove_components_mask_index.insert(next_mask, Box::new(|world, guid|{
            // let s: &RwLock<<C as ::Component>::Storage> = any.downcast_ref().unwrap();
            // s.write().unwrap().remove(guid)
            world.storage_mut::<C>().unwrap().remove(guid);
        }));
        self.storages.insert(type_id, storage);
    }

    pub fn register_thread_local<C: ComponentThreadLocal>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RefCell::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask_mut = self.next_component_mask.get_mut();
        let next_mask = *next_mask_mut;
        *next_mask_mut *= 2;
        self.components_mask_index.insert(type_id, next_mask);
        self.reverse_components_mask_index.insert(next_mask, type_id);
        self.remove_components_mask_index.insert(next_mask, Box::new(|world, guid|{
            //let s: &RefCell<<C as ::Component>::Storage> = any.downcast_ref().unwrap();
            //s.borrow_mut().remove(guid)
            world.storage_mut::<C>().unwrap().remove(guid);

        }));
        self.storages_thread_local.insert(type_id, storage);
    }

    pub fn create_entity(&mut self) -> EntityBuilder{
        self.entities_index_per_mask.get_mut().unwrap().clear();
        EntityBuilder::new(self)
    }

    pub fn entities(&self) -> Entities{
        Entities::new(self)
    }

    pub fn entities_thread_local(&self) -> EntitiesThreadLocal{
        EntitiesThreadLocal::new(self)
    }

    pub fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        {
            let storage = self.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.remove(entity.guid())
            }else{
                panic!("Trying to add component of type {} without registering first", "C::type_name()")
            }
        }
        self.entities[entity.guid()].components_mask &= !self.components_mask_index[&TypeId::of::<C>()];
        let mask = self.components_mask::<C>();
        let type_id = self.reverse_components_mask_index[&mask];
        if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(&type_id){
            cache.clear();
        }
        self.entities_index_per_mask.write().unwrap().clear();
    }

    pub fn remove_entity(&mut self, entity: &::Entity){
        //if let Ok(pos) = self.entities.binary_search_by(|e| e.guid().cmp(&entity.guid())){
        let entity = unsafe{ mem::transmute::<&mut Entity, &mut Entity>(&mut self.entities[entity.guid()]) };
        let mut mask = 1;
        while mask < self.next_component_mask.load(Ordering::Relaxed){
            if entity.components_mask & mask == mask{
                // let storage = &self.storages[&type_id];
                let remove_component = unsafe{
                    mem::transmute::<&Box<Fn(&World, usize)>, &Box<Fn(&World, usize)>>(&self.remove_components_mask_index[&mask])
                };
                remove_component(self, entity.guid());
                entity.components_mask &= !mask;
                let type_id = self.reverse_components_mask_index[&mask];
                if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(&type_id){
                    cache.clear();
                }
            }
            mask *= 2;
        }
        // self.ordered_entities_index_per_mask.write().unwrap().clear();
        self.entities_index_per_mask.write().unwrap().clear();
        // TODO: can't remove entities since we rely on order for fast entitty search
        // mostly on ordered_ids_for. others are add / remove component which could be slower
        // without problem
        // Use DenseVec for entities storage?
        // self.entities.remove(pos);
    }

    pub(crate) fn next_guid(&mut self) -> usize{
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn push_entity(&mut self, e: ::Entity){
        self.entities.push(e)
    }

    pub(crate) fn storage<C: ::Component>(&self) -> Option<RwLockReadGuard<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.read().unwrap()
        })
    }

    pub(crate) fn storage_mut<C: ::Component>(&self) -> Option<RwLockWriteGuard<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.write().unwrap()
        })
    }

    pub(crate) fn storage_thread_local<C: ::Component>(&self) -> Option<ReadGuardRef<<C as ::Component>::Storage>> {
        let local = self.storages_thread_local.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            ReadGuard::ThreadLocal(s.borrow())
        });
        if local.is_some(){
            local.map(|local| ReadGuardRef::new(local))
        }else{
            self.storage::<C>().map(|sync| ReadGuardRef::new(ReadGuard::Sync(sync)))
        }
    }

    pub(crate) fn storage_thread_local_mut<C: ::Component>(&self) -> Option<WriteGuardRef<<C as ::Component>::Storage>> {
        let local = self.storages_thread_local.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            WriteGuard::ThreadLocal(s.borrow_mut())
        });
        if local.is_some(){
            local.map(|local| WriteGuardRef::new(local))
        }else{
            self.storage_mut::<C>().map(|sync| WriteGuardRef::new(WriteGuard::Sync(sync)))
        }
    }

    pub(crate) fn components_mask<C: ::Component>(&self) -> usize{
        self.components_mask_index[&TypeId::of::<C>()]
    }

    pub(crate) fn entities_for_mask(&self, mask: usize) -> IndexGuard{
        if !self.entities_index_per_mask.read().unwrap().contains_key(&mask){
            let entities = self.entities.iter().filter_map(|e|
                if e.components_mask & mask == mask{
                    Some(e.guid())
                }else{
                    None
                }).collect();
            self.entities_index_per_mask.write().unwrap().insert(mask, entities);
        }
        let _index_guard = self.entities_index_per_mask.read().unwrap();
        let ptr = _index_guard[&mask].as_ptr();
        let len = _index_guard[&mask].len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }

    pub(crate) fn ordered_entities_for<'a, C: Component>(&self, mask: usize) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<C>{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){
            let entities = self.storage::<C>()
                .expect(&format!("Trying to use non registered type {}", "C::type_name()"))
                .ordered_ids()
                .into_iter()
                .map(|i| *i)
                .filter(|i| self.entities[*i].components_mask & mask == mask)
                .collect();
            self.entities_index_per_mask.write().unwrap().insert(mask, entities);
        }
        let _index_guard = self.entities_index_per_mask.read().unwrap();
        let ptr = _index_guard[&mask].as_ptr();
        let len = _index_guard[&mask].len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }

    pub(crate) fn thread_local_ordered_entities_for<'a, C: Component>(&self, mask: usize) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<C>{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){
            let entities = self.storage_thread_local::<C>()
                .expect(&format!("Trying to use non registered type {}", "C::type_name()"))
                .ordered_ids()
                .into_iter()
                .map(|i| *i)
                .filter(|i| self.entities[*i].components_mask & mask == mask)
                .collect();
            self.entities_index_per_mask.write().unwrap().insert(mask, entities);
        }
        let _index_guard = self.entities_index_per_mask.read().unwrap();
        let ptr = _index_guard[&mask].as_ptr();
        let len = _index_guard[&mask].len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }
}
