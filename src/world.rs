use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::cell::{RefCell, Ref, RefMut, UnsafeCell};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::mem;
use std::u32;

use ::Entity;
use component::{ComponentSync, Component, ComponentThreadLocal, OneToNComponentSync, OneToNComponentThreadLocal};
use storage::{Storage, HierarchicalStorage, OneToNStorage};
use entity::{EntityBuilder, Entities, EntitiesThreadLocal};
use sync::*;
use rayon::prelude::*;

pub struct World{
    storages: HashMap<TypeId, Box<Any>>,
    storages_thread_local: HashMap<TypeId, Box<Any>>,
    resources: HashMap<TypeId, Box<Any>>,

    next_guid: AtomicUsize,
    entities: Vec<Entity>, // Doesn't need lock cause never accesed mut from Entities?
    entities_index_per_mask: UnsafeCell<HashMap<usize, RwLock<Vec<usize>>>>,
    entities_index_per_mask_guard: RwLock<()>,
    ordered_entities_index_per_mask: RwLock<HashMap<TypeId, HashMap<usize, Vec<usize>>>>,
    reverse_components_mask_index: HashMap<usize, TypeId>,
    remove_components_mask_index: HashMap<usize, Box<Fn(&World, usize)>>,

    next_component_mask: AtomicUsize,
    pub(crate) components_mask_index: HashMap<TypeId, usize>,


    systems: Vec<(usize, SyncSystem)>,
    systems_thread_local: Vec<(usize, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
    barriers: Vec<(usize)>,
    next_system_priority: AtomicUsize,
}

impl World{
    pub fn new() -> World{
        World{
            storages: HashMap::new(),
            storages_thread_local: HashMap::new(),
            resources: HashMap::new(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: AtomicUsize::new(1),
            entities: Vec::new(),
            components_mask_index: HashMap::new(),
            entities_index_per_mask_guard: RwLock::new(()),
            entities_index_per_mask: UnsafeCell::new(HashMap::new()),
            ordered_entities_index_per_mask: RwLock::new(HashMap::new()),
            reverse_components_mask_index: HashMap::new(),
            remove_components_mask_index: HashMap::new(),
            systems: vec![],
            systems_thread_local: vec![],
            barriers: vec![],
            next_system_priority: AtomicUsize::new(0),
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
        self.clear_entities_per_mask_index();
        EntityBuilder::new(self)
    }

    pub fn entities(&self) -> Entities{
        Entities::new(self)
    }

    pub fn entities_thread_local(&self) -> EntitiesThreadLocal{
        EntitiesThreadLocal::new(self)
    }

    pub fn add_component_to<C: ComponentSync>(&mut self, entity: &Entity, component: C){
        self.clear_entities_per_mask_index();
        {
            let storage = self.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(entity.guid(), component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        let entity = &mut self.entities[entity.guid()];
        entity.components_mask |= self.components_mask_index[&TypeId::of::<C>()];
    }

    pub fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C){
        self.clear_entities_per_mask_index();
        {
            let storage = self.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(entity.guid(), component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        let entity = &mut self.entities[entity.guid()];
        entity.components_mask |= self.components_mask_index[&TypeId::of::<C>()];
    }

    pub fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        {
            let storage = self.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(entity.guid(), component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        let entity = &mut self.entities[entity.guid()];
        entity.components_mask |= self.components_mask_index[&TypeId::of::<C>()];
    }

    pub fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        {
            let storage = self.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(entity.guid(), component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        let entity = &mut self.entities[entity.guid()];
        entity.components_mask |= self.components_mask_index[&TypeId::of::<C>()];
    }

    pub fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        {
            let storage = self.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.remove(entity.guid())
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        }
        self.entities[entity.guid()].components_mask &= !self.components_mask_index[&TypeId::of::<C>()];
        let mask = self.components_mask::<C>();
        let type_id = self.reverse_components_mask_index[&mask];
        if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(&type_id){
            cache.clear();
        }
        self.clear_entities_per_mask_index();
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
        self.clear_entities_per_mask_index()
        // TODO: can't remove entities since we rely on order for fast entitty search
        // mostly on ordered_ids_for. others are add / remove component which could be slower
        // without problem
        // Use DenseVec for entities storage?
        // self.entities.remove(pos);
    }

    pub fn add_resource<T: 'static + Send>(&mut self, resource: T){
        self.resources.insert(TypeId::of::<T>(), Box::new(RefCell::new(resource)) as Box<Any>);
    }

    pub fn resource<T: 'static + Send>(&self) -> Option<Ref<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow()
        })
    }

    pub fn resource_mut<T: 'static + Send>(&self) -> Option<RefMut<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow_mut()
        })
    }

    pub fn resources(&self) -> ::Resources{
        ::Resources::new(self)
    }


    pub fn add_resource_thread_local<T: 'static>(&mut self, resource: T){
        self.resources.insert(TypeId::of::<T>(), Box::new(RefCell::new(resource)) as Box<Any>);
    }

    pub fn resource_thread_local<T: 'static>(&self) -> Option<Ref<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow()
        })
    }

    pub fn resource_thread_local_mut<T: 'static>(&self) -> Option<RefMut<T>>{
        self.resources.get(&TypeId::of::<T>()).map(|t| {
            let t: &RefCell<T> = t.downcast_ref().unwrap();
            t.borrow_mut()
        })
    }

    pub fn resources_thread_local(&self) -> ::ResourcesThreadLocal{
        ::ResourcesThreadLocal::new(self)
    }

    pub fn add_system<S>(&mut self, system: S) -> &mut World
    where  for<'a> S: ::System<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems.push((prio, SyncSystem::new(system)));
        self
    }

    pub fn add_system_thread_local<S>(&mut self, system: S) -> &mut World
    where  for<'a> S: ::SystemThreadLocal<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems_thread_local.push((prio, Box::new(system)));
        self
    }

    pub fn add_barrier(&mut self) -> &mut World
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.barriers.push(prio);
        self
    }

    pub fn run_once(&mut self){
        let systems_thread_local = unsafe{ mem::transmute::<
                &mut Vec<(usize, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
                &mut Vec<(usize, Box<for<'a> ::system::SystemThreadLocal<'a>>)>
            >(&mut self.systems_thread_local) };
        let mut i_systems = 0;
        let mut i_systems_tl = 0;
        let mut i_barriers = 0;
        let entities = self.entities();
        let resources = self.resources();
        loop {
            let next_system = self.systems.get(i_systems);
            let next_system_tl = systems_thread_local.get_mut(i_systems_tl);
            let next_barrier = match self.barriers.get(i_barriers){
                Some(barrier) => *barrier as u32,
                None => u32::MAX,
            };
            match (next_system, next_system_tl){
                (Some(&(sys_prio, ref system)), Some(&mut(sys_tl_prio, ref mut system_tl))) => {
                    if sys_prio < sys_tl_prio {
                        let mut parallel_systems = vec![(sys_prio, system)];
                        i_systems += 1;
                        while let Some(&(sys_prio, ref system)) = self.systems.get(i_systems){
                            if sys_prio < sys_tl_prio  && sys_prio < next_barrier as usize{
                                parallel_systems.push((sys_prio, system));
                                i_systems += 1;
                            }else{
                                if sys_prio > next_barrier as usize{
                                    i_barriers += 1;
                                }
                                break;
                            }
                        }
                        parallel_systems.par_iter().for_each(|&(_, s)| {
                            s.borrow_mut().run(entities, resources)
                        });
                    }else{
                        system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        i_systems_tl += 1;
                    }
                }
                (Some(&(sys_prio, ref system)), None) => {
                    let mut parallel_systems = vec![(sys_prio, system)];
                    i_systems += 1;
                    while let Some(&(sys_prio, ref system)) = self.systems.get(i_systems){
                        if sys_prio < next_barrier as usize{
                            parallel_systems.push((sys_prio, system));
                            i_systems += 1;
                        }else{
                            if sys_prio > next_barrier as usize{
                                i_barriers += 1;
                            }
                            break;
                        }
                    }

                    parallel_systems.par_iter().for_each(|&(_, s)| {
                        s.borrow_mut().run(entities, resources)
                    });
                }
                (None, Some(&mut(_, ref mut system_tl))) => {
                    system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                    i_systems_tl += 1;
                }
                (None, None) => break
            }
        }
    }

    fn clear_entities_per_mask_index(&mut self){
        unsafe{
            let _guard = self.entities_index_per_mask_guard.write().unwrap();
            (*self.entities_index_per_mask.get()).clear();
        }
    }

    pub(crate) fn entities_ref(&self) -> &[Entity]{
        &self.entities
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
        *self.components_mask_index.get(&TypeId::of::<C>())
            .expect(&format!("Trying to use component {} before registering", C::type_name()))
    }

    pub(crate) fn entities_for_mask(&self, mask: usize) -> IndexGuard{
        let contains_key = unsafe {
            let _guard = self.entities_index_per_mask_guard.read().unwrap();
            (*self.entities_index_per_mask.get()).contains_key(&mask)
        };
        if !contains_key {
            let entities = self.entities.iter().filter_map(|e|
                if e.components_mask & mask == mask{
                    Some(e.guid())
                }else{
                    None
                }).collect::<Vec<_>>();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask, RwLock::new(entities));
            }
        }
        let _index_guard = unsafe{
            let _guard = self.entities_index_per_mask_guard.read().unwrap();
            (*self.entities_index_per_mask.get()).get(&mask).unwrap().read().unwrap()
        };
        let ptr = _index_guard.as_ptr();
        let len = _index_guard.len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }

    pub(crate) fn ordered_entities_for<'a, C: Component>(&self, mask: usize) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<'a,C>{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){
            let entities = self.storage::<C>()
                .expect(&format!("Trying to use non registered type {}", C::type_name()))
                .ordered_ids()
                .into_iter()
                .map(|i| *i)
                .filter(|i| self.entities[*i].components_mask & mask == mask)
                .collect();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask, RwLock::new(entities));
            }
        }
        let _index_guard = unsafe{
            let _guard = self.entities_index_per_mask_guard.read().unwrap();
            (*self.entities_index_per_mask.get()).get(&mask).unwrap().read().unwrap()
        };
        let ptr = _index_guard.as_ptr();
        let len = _index_guard.len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }

    pub(crate) fn thread_local_ordered_entities_for<'a, C: Component>(&self, mask: usize) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<'a,C>{
        if !self.ordered_entities_index_per_mask.write().unwrap().entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){
            let entities = self.storage_thread_local::<C>()
                .expect(&format!("Trying to use non registered type {}", C::type_name()))
                .ordered_ids()
                .into_iter()
                .map(|i| *i)
                .filter(|i| self.entities[*i].components_mask & mask == mask)
                .collect();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask, RwLock::new(entities));
            }
        }
        let _guard = self.entities_index_per_mask_guard.read().unwrap();
        let _index_guard = unsafe{
            let _guard = self.entities_index_per_mask_guard.read().unwrap();
            (*self.entities_index_per_mask.get()).get(&mask).unwrap().read().unwrap()
        };
        let ptr = _index_guard.as_ptr();
        let len = _index_guard.len();
        let index = unsafe{ slice::from_raw_parts(ptr, len) };
        IndexGuard{
            _index_guard,
            index,
        }
    }
}


struct SyncSystem(UnsafeCell<Box<for<'a> ::system::System<'a>>>);

impl SyncSystem{
    fn new<S: for<'a> ::system::System<'a> + 'static>(s: S) -> SyncSystem{
        SyncSystem(UnsafeCell::new(Box::new(s)))
    }

    fn borrow_mut(&self) -> &mut Box<for<'a> ::system::System<'a>>{
        unsafe{ &mut *self.0.get() }
    }

    fn _borrow(&self) -> &Box<for<'a> ::system::System<'a>>{
        unsafe{ &*self.0.get() }
    }
}

unsafe impl Send for SyncSystem{}
unsafe impl Sync for SyncSystem{}
