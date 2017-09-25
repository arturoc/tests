use std::any::{Any, TypeId};
use std::cell::{RefCell, Ref, RefMut};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::mem;

use ::Entity;
use component::{ComponentSync, Component, ComponentThreadLocal};
use storage::{Storage, AnyStorage, HierarchicalStorage};
use entity::{EntityBuilder, Entities, EntitiesThreadLocal};
use sync::*;
use fnv::FnvHashMap;

pub struct World<'a>{
    storages: FnvHashMap<TypeId, Box<AnyStorage + 'a>>,
    storages_thread_local: FnvHashMap<TypeId, Box<AnyStorage + 'a>>,
    resources: FnvHashMap<TypeId, Box<Any>>,

    next_guid: AtomicUsize,
    entities: Vec<Entity>, // Doesn't need lock cause never accesed mut from Entities?
    entities_index_per_mask: RwLock<FnvHashMap<usize, Vec<usize>>>,
    ordered_entities_index_per_mask: RwLock<FnvHashMap<TypeId, FnvHashMap<usize, Vec<usize>>>>,
    reverse_components_mask_index: FnvHashMap<usize, TypeId>,
    remove_components_mask_index: FnvHashMap<usize, Box<for<'b> Fn(&'b World<'a>, usize)>>,

    next_component_mask: AtomicUsize,
    pub(crate) components_mask_index: FnvHashMap<TypeId, usize>,
}

impl<'a> World<'a>{
    pub fn new() -> World<'a>{
        World{
            storages: FnvHashMap::default(),
            storages_thread_local: FnvHashMap::default(),
            resources: FnvHashMap::default(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: AtomicUsize::new(1),
            entities: Vec::new(),
            components_mask_index: FnvHashMap::default(),
            entities_index_per_mask: RwLock::new(FnvHashMap::default()),
            ordered_entities_index_per_mask: RwLock::new(FnvHashMap::default()),
            reverse_components_mask_index: FnvHashMap::default(),
            remove_components_mask_index: FnvHashMap::default(),
        }
    }

    pub fn register<C: ComponentSync<'a>>(&mut self) {
        let type_id = C::type_id();
        let storage = Box::new(RwLock::new(<C as Component<'a>>::Storage::new())) as Box<AnyStorage + 'a>;
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

    pub fn register_thread_local<C: ComponentThreadLocal<'a>>(&mut self) {
        let type_id = C::type_id();
        let storage = Box::new(RefCell::new(<C as Component<'a>>::Storage::new())) as Box<AnyStorage + 'a>;
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

    pub fn create_entity<'b>(&'b mut self) -> EntityBuilder<'a, 'b>{
        self.entities_index_per_mask.get_mut().unwrap().clear();
        EntityBuilder::new(self)
    }

    pub fn entities<'b>(&'b self) -> Entities<'a, 'b>{
        Entities::new(self)
    }

    pub fn entities_thread_local<'b>(&'b self) -> EntitiesThreadLocal<'a, 'b>{
        EntitiesThreadLocal::new(self)
    }

    pub(crate) fn entities_ref(&self) -> &[Entity]{
        &self.entities
    }

    pub fn remove_component_from<C: ::Component<'a>>(&mut self, entity: &::Entity){
        {
            let storage = self.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.remove(entity.guid())
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        }
        self.entities[entity.guid()].components_mask &= !self.components_mask_index[&C::type_id()];
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
                let remove_component = &self.remove_components_mask_index[&mask];
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

    pub fn add_resource<T: 'static>(&mut self, resource: T){
        self.resources.insert(TypeId::of::<T>(), Box::new(RefCell::new(resource)) as Box<Any>);
    }

    pub fn resource<T: 'static>(&self) -> Option<Ref<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow()
        })
    }

    pub fn resource_mut<T: 'static>(&self) -> Option<RefMut<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow_mut()
        })
    }

    pub(crate) fn next_guid(&mut self) -> usize{
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn push_entity(&mut self, e: ::Entity){
        self.entities.push(e)
    }

    pub(crate) fn storage<C: ::Component<'a>>(&self) -> Option<RwLockReadGuard<'a, <C as ::Component<'a>>::Storage>> {
        let world = unsafe{ mem::transmute::<&World<'a>, &World<'a>>(self) };
        world.storages.get(&C::type_id()).map(|s| {
            let s: &'a AnyStorage = &**s;
            let s: &RwLock<<C as ::Component<'a>>::Storage> = unsafe{ &*(s as *const AnyStorage as *const RwLock<<C as ::Component<'a>>::Storage>) };
            s.read().unwrap()
        })
    }

    pub(crate) fn storage_mut<C: ::Component<'a>>(&self) -> Option<RwLockWriteGuard<'a, <C as ::Component<'a>>::Storage>> {
        let world = unsafe{ mem::transmute::<&World<'a>, &World<'a>>(self) };
        world.storages.get(&C::type_id()).map(|s| {
            let s: &'a AnyStorage = &**s;
            let s: &RwLock<<C as ::Component<'a>>::Storage> = unsafe{ &*(s as *const AnyStorage as *const RwLock<<C as ::Component<'a>>::Storage>) };
            s.write().unwrap()
        })
    }

    pub(crate) fn storage_thread_local<C: ::Component<'a>>(&self) -> Option<ReadGuardRef<'a, <C as ::Component<'a>>::Storage>> {
        let world = unsafe{ mem::transmute::<&World<'a>, &World<'a>>(self) };
        let local = world.storages_thread_local.get(&C::type_id()).map(|s| {
            let s: &'a AnyStorage = &**s;
            let s: &RefCell<<C as ::Component<'a>>::Storage> = unsafe{ &*(s as *const AnyStorage as *const RefCell<<C as ::Component<'a>>::Storage>) };
            ReadGuard::ThreadLocal(s.borrow())
        });
        if local.is_some(){
            local.map(|local| ReadGuardRef::new(local))
        }else{
            self.storage::<C>().map(|sync| ReadGuardRef::new(ReadGuard::Sync(sync)))
        }
    }

    pub(crate) fn storage_thread_local_mut<C: ::Component<'a>>(&self) -> Option<WriteGuardRef<'a, <C as ::Component<'a>>::Storage>> {
        let world = unsafe{ mem::transmute::<&World<'a>, &World<'a>>(self) };
        let local = world.storages_thread_local.get(&C::type_id()).map(|s| {
            let s: &'a AnyStorage = &**s;
            let s: &RefCell<<C as ::Component<'a>>::Storage> = unsafe{ &*(s as *const AnyStorage as *const RefCell<<C as ::Component<'a>>::Storage>) };
            WriteGuard::ThreadLocal(s.borrow_mut())
        });
        if local.is_some(){
            local.map(|local| WriteGuardRef::new(local))
        }else{
            self.storage_mut::<C>().map(|sync| WriteGuardRef::new(WriteGuard::Sync(sync)))
        }
    }

    pub(crate) fn components_mask<C: ::Component<'a>>(&self) -> usize{
        self.components_mask_index[&C::type_id()]
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

    pub(crate) fn ordered_entities_for<C: Component<'a>>(&self, mask: usize) -> IndexGuard
        where <C as Component<'a>>::Storage: ::HierarchicalStorage<'a,C>,
              RwLock<<C as Component<'a>>::Storage>: AnyStorage{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(C::type_id())
            .or_insert_with(|| FnvHashMap::default())
            .contains_key(&mask){
            let entities = self.storage::<C>()
                .expect(&format!("Trying to use non registered type {}", C::type_name()))
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

    pub(crate) fn thread_local_ordered_entities_for<C: Component<'a>>(&self, mask: usize) -> IndexGuard
        where <C as Component<'a>>::Storage: ::HierarchicalStorage<'a,C>,
              RwLock<<C as Component<'a>>::Storage>: AnyStorage{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(C::type_id())
            .or_insert_with(|| FnvHashMap::default())
            .contains_key(&mask){
            let entities = self.storage_thread_local::<C>()
                .expect(&format!("Trying to use non registered type {}", C::type_name()))
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
