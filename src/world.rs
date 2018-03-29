use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::cell::{RefCell, Ref, RefMut, UnsafeCell};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::mem;
use std::u32;
use std::thread;

use ::Entity;
use component::{ComponentSync, Component, ComponentThreadLocal,
    OneToNComponentSync, OneToNComponentThreadLocal};
use storage::{Storage, HierarchicalStorage, OneToNStorage};
use entity::{EntityBuilder, Entities, EntitiesThreadLocal};
use sync::*;
use rayon::prelude::*;
use ::{Bitmask, MaskType, NextMask};


#[cfg(feature="stats_events")]
use seitan::*;
#[cfg(feature="stats_events")]
use std::time;


pub struct World{
    storages: HashMap<TypeId, Box<Any>>,
    storages_thread_local: HashMap<TypeId, Box<Any>>,
    resources: HashMap<TypeId, Box<Any>>,

    next_guid: AtomicUsize,
    entities: Vec<(Entity, ::MaskType)>, // Doesn't need lock cause never accesed mut from Entities?
    entities_index_per_mask: UnsafeCell<HashMap<Bitmask, RwLock<Vec<usize>>>>,
    entities_index_per_mask_guard: RwLock<()>,
    ordered_entities_index_per_mask: RwLock<HashMap<TypeId, HashMap<Bitmask, Vec<usize>>>>,
    reverse_components_mask_index: HashMap<MaskType, TypeId>,
    remove_components_mask_index: HashMap<MaskType, Box<Fn(&World, usize)>>,

    next_component_mask: NextMask,
    pub(crate) components_mask_index: HashMap<TypeId, MaskType>,


    systems: Vec<(usize, String, SyncSystem)>,
    systems_thread_local: Vec<(usize, String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
    barriers: Vec<(usize)>,
    next_system_priority: AtomicUsize,

    #[cfg(feature="stats_events")]
    stats: Vec<(String, time::Duration)>,

    #[cfg(feature="stats_events")]
    stats_events: HashMap<String, SenderRc<'static, time::Duration>>,
}

unsafe impl Send for World{}

impl World{
    pub fn new() -> World{
        World{
            storages: HashMap::new(),
            storages_thread_local: HashMap::new(),

            resources: HashMap::new(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: NextMask::new(),
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

            #[cfg(feature="stats_events")]
            stats: Vec::new(),

            #[cfg(feature="stats_events")]
            stats_events: HashMap::new(),
        }
    }

    pub fn register<C: ComponentSync>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RwLock::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask = self.next_component_mask.next();
        self.components_mask_index.insert(type_id, next_mask.clone());
        self.reverse_components_mask_index.insert(next_mask.clone(), type_id);
        self.storages.insert(type_id, storage);
        self.remove_components_mask_index.insert(next_mask, Box::new(move |world, guid|{
            // let s: &RwLock<<C as ::Component>::Storage> = any.downcast_ref().unwrap();
            // s.write().unwrap().remove(guid)

            world.storage_mut::<C>()
                .expect(&format!("Trying to delete component {} without registering first", C::type_name()))
                .remove(guid);
        }));
    }

    pub fn register_thread_local<C: ComponentThreadLocal>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RefCell::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask = self.next_component_mask.next();
        self.components_mask_index.insert(type_id, next_mask.clone());
        self.reverse_components_mask_index.insert(next_mask.clone(), type_id);
        self.storages_thread_local.insert(type_id, storage);
        self.remove_components_mask_index.insert(next_mask, Box::new(move |world, guid|{
            //let s: &RefCell<<C as ::Component>::Storage> = any.downcast_ref().unwrap();
            //s.borrow_mut().remove(guid)

            world.storage_thread_local_mut::<C>()
                .expect(&format!("Trying to delete component {} without registering first", C::type_name()))
                .remove(guid);

        }));
    }

    pub fn create_entity(&mut self) -> EntityBuilder{
        self.clear_entities_per_mask_index();
        EntityBuilder::new(self)
    }

    pub fn entities<'a>(&'a self) -> Entities<'a>{
        Entities::new(self)
    }

    pub fn entities_thread_local<'a>(&'a self) -> EntitiesThreadLocal<'a>{
        EntitiesThreadLocal::new(self)
    }

    pub fn add_component_to<C: ComponentSync>(&mut self, entity: &Entity, component: C){
        self.clear_entities_per_mask_index();
        self.storage_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&TypeId::of::<C>()].clone();
    }

    pub fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C){
        self.clear_entities_per_mask_index();
        self.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&TypeId::of::<C>()].clone();
    }

    pub fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        self.storage_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert_slice(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&TypeId::of::<C>()].clone();
    }

    pub fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        self.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert_slice(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&TypeId::of::<C>()].clone();
    }

    pub fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        self.storage_mut::<C>()
            .expect(&format!("Trying to remove component of type {} without registering first", C::type_name()))
            .remove(entity.guid());
        self.entities[entity.guid()].1 ^= self.components_mask_index[&TypeId::of::<C>()].clone();
        let mask = self.components_mask::<C>();
        let type_id = self.reverse_components_mask_index[&mask];
        if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(&type_id){
            cache.clear();
        }
        self.clear_entities_per_mask_index();
    }

    pub fn remove_entity(&mut self, entity: &::Entity){
        //if let Ok(pos) = self.entities.binary_search_by(|e| e.guid().cmp(&entity.guid())){
        let entity_mask = unsafe{ mem::transmute::<&mut ::MaskType, &mut ::MaskType>(&mut self.entities[entity.guid()].1) };
        let mut mask = MaskType::from(1usize);
        while mask < self.next_component_mask.get(){
            if entity_mask.clone() & mask.clone() == mask{
                // let storage = &self.storages[&type_id];
                let remove_component = unsafe{
                    mem::transmute::<&Box<Fn(&World, usize)>, &Box<Fn(&World, usize)>>(&self.remove_components_mask_index[&mask])
                };
                remove_component(self, entity.guid());
                *entity_mask ^= mask.clone();
                let type_id = self.reverse_components_mask_index[&mask];
                if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(&type_id){
                    cache.clear();
                }
            }
            mask *= MaskType::from(2usize);
        }
        // self.ordered_entities_index_per_mask.write().unwrap().clear();
        self.clear_entities_per_mask_index()
        // TODO: can't remove entities since we rely on order for fast entitty search
        // mostly on ordered_ids_for. others are add / remove component which could be slower
        // without problem
        // Use DenseVec for entities storage?
        // self.entities.remove(pos);
        // self.masks.remove(pos)
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

    pub fn remove_resource<T: 'static + Send>(&mut self) -> Option<T> {
        self.resources.remove(&TypeId::of::<T>()).map(|t| {
            let t: Box<RefCell<T>> = t.downcast().unwrap();
            t.into_inner()
        })
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

    pub fn remove_resource_thread_local<T: 'static>(&mut self) -> Option<T> {
        self.resources.remove(&TypeId::of::<T>()).map(|t| {
            let t: Box<RefCell<T>> = t.downcast().unwrap();
            t.into_inner()
        })
    }

    pub fn add_system<S>(&mut self, system: S) -> &mut World
    where  for<'a> S: ::System<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems.push((prio, String::new(), SyncSystem::new(system)));
        self
    }

    pub fn add_system_thread_local<S>(&mut self, system: S) -> &mut World
    where  for<'a> S: ::SystemThreadLocal<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems_thread_local.push((prio, String::new(), Box::new(system)));
        self
    }

    #[cfg(feature="stats_events")]
    pub fn add_system_with_stats<S>(&mut self, system: S, name: &str) -> &mut World
    where  for<'a> S: ::System<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems.push((prio, name.to_owned(), SyncSystem::new(system)));
        self.stats_events.insert(name.to_owned(), SenderRc::new());
        self
    }

    #[cfg(feature="stats_events")]
    pub fn add_system_with_stats_thread_local<S>(&mut self, system: S, name: &str) -> &mut World
    where  for<'a> S: ::SystemThreadLocal<'a> + 'static
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.systems_thread_local.push((prio, name.to_owned(), Box::new(system)));
        self.stats_events.insert(name.to_owned(), SenderRc::new());
        self
    }

    #[cfg(feature="stats_events")]
    pub fn stats(&mut self) -> Vec<(String, Property<'static, time::Duration>)>{
        self.stats_events.iter_mut()
            .map(|(name, sender)| (name.clone(), sender.stream().to_property(time::Duration::new(0, 0))))
            .collect()
    }

    pub fn add_barrier(&mut self) -> &mut World
    {
        let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.barriers.push(prio);
        self
    }

    pub fn run_once(&mut self){
        let systems_thread_local = unsafe{ mem::transmute::<
                &mut Vec<(usize, String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
                &mut Vec<(usize, String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>
            >(&mut self.systems_thread_local) };
        let systems_thread_local2 = unsafe{ mem::transmute::<
                &mut Vec<(usize, String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
                &mut Vec<(usize, String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>
            >(&mut self.systems_thread_local) };


        #[cfg(feature="stats_events")]
        let stats = {
            self.stats.reserve(self.systems.len() + self.systems_thread_local.len());
            self.stats.clear();
            unsafe{ mem::transmute::<
                    &mut Vec<(String, time::Duration)>,
                    &mut Vec<(String, time::Duration)>
                >(&mut self.stats)}
        };

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
                (Some(&(sys_prio, ref name, ref system)), Some(&mut(sys_tl_prio, ref name_tl, ref mut system_tl))) => {
                    if sys_prio < sys_tl_prio {
                        let mut parallel_systems = vec![(sys_prio, name.clone(), system)];
                        i_systems += 1;
                        while let Some(&(sys_prio, ref name, ref system)) = self.systems.get(i_systems){
                            if sys_prio < sys_tl_prio  && sys_prio < next_barrier as usize{
                                parallel_systems.push((sys_prio, name.clone(), system));
                                i_systems += 1;
                            }else{
                                if sys_prio > next_barrier as usize{
                                    i_barriers += 1;
                                }
                                break;
                            }
                        }


                        #[cfg(feature="stats_events")]
                        stats.par_extend(parallel_systems.par_iter().filter_map(|&(_, ref name, s)| {
                            if name != "" {
                                let then = time::Instant::now();
                                s.borrow_mut().run(entities, resources);
                                let now = time::Instant::now();
                                Some((name.clone(), now - then))
                            }else{
                                system.borrow_mut().run(entities, resources);
                                None
                            }
                        }));

                        #[cfg(not(feature="stats_events"))]
                        parallel_systems.par_iter().for_each(|&(_, _, s)| {
                            s.borrow_mut().run(entities, resources)
                        });

                        // Run next tl systems in parallel but on the main thread
                        // not possible with rayon atm? we need to async wait for the
                        // parallel systems to finish
                        // if sys_tl_prio < next_barrier as usize {
                        //     #[cfg(feature="stats_events")]
                        //     {
                        //         if name_tl != "" {
                        //             let then = time::Instant::now();
                        //             system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        //             let now = time::Instant::now();
                        //             stats.push((name_tl.clone(), now - then));
                        //         }else{
                        //             system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        //         }
                        //     }

                        //     #[cfg(not(feature="stats_events"))]
                        //     system_tl.run(self.entities_thread_local(), self.resources_thread_local());

                        //     while let Some(&mut (sys_tl_prio, ref name_tl, ref mut system_tl)) = systems_thread_local2.get_mut(i_systems_tl){
                        //         if sys_tl_prio < next_barrier as usize{
                        //             #[cfg(feature="stats_events")]
                        //             {
                        //                 if name_tl != "" {
                        //                     let then = time::Instant::now();
                        //                     system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        //                     let now = time::Instant::now();
                        //                     stats.push((name_tl.clone(), now - then));
                        //                 }else{
                        //                     system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        //                 }
                        //             }

                        //             #[cfg(not(feature="stats_events"))]
                        //             system_tl.run(self.entities_thread_local(), self.resources_thread_local());

                        //             i_systems_tl += 1;
                        //         }else{
                        //             if sys_prio > next_barrier as usize{
                        //                 i_barriers += 1;
                        //             }
                        //             break;
                        //         }
                        //     }
                        // }
                    }else{
                        #[cfg(feature="stats_events")]
                        {
                            if name_tl != "" {
                                let then = time::Instant::now();
                                system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                                let now = time::Instant::now();
                                stats.push((name_tl.clone(), now - then));
                            }else{
                                system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                            }
                        }

                        #[cfg(not(feature="stats_events"))]
                        system_tl.run(self.entities_thread_local(), self.resources_thread_local());

                        i_systems_tl += 1;
                    }
                }
                (Some(&(sys_prio, ref name, ref system)), None) => {
                    let mut parallel_systems = vec![(sys_prio, name.clone(), system)];
                    i_systems += 1;
                    while let Some(&(sys_prio, ref name, ref system)) = self.systems.get(i_systems){
                        if sys_prio < next_barrier as usize{
                            parallel_systems.push((sys_prio, name.clone(), system));
                            i_systems += 1;
                        }else{
                            if sys_prio > next_barrier as usize{
                                i_barriers += 1;
                            }
                            break;
                        }
                    }


                    #[cfg(feature="stats_events")]
                    stats.par_extend(parallel_systems.par_iter().filter_map(|&(_, ref name, s)| {
                        if name != "" {
                            let then = time::Instant::now();
                            s.borrow_mut().run(entities, resources);
                            let now = time::Instant::now();
                            Some((name.clone(), now - then))
                        }else{
                            system.borrow_mut().run(entities, resources);
                            None
                        }
                    }));

                    #[cfg(not(feature="stats_events"))]
                    parallel_systems.par_iter().for_each(|&(_, _, s)| {
                        s.borrow_mut().run(entities, resources)
                    });
                }
                (None, Some(&mut(_, ref _name, ref mut system_tl))) => {
                    #[cfg(feature="stats_events")]
                    {
                        if _name != "" {
                            let then = time::Instant::now();
                            system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                            let now = time::Instant::now();
                            stats.push((_name.clone(), now - then));
                        }else{
                            system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                        }
                    }

                    #[cfg(not(feature="stats_events"))]
                    system_tl.run(self.entities_thread_local(), self.resources_thread_local());
                    i_systems_tl += 1;
                }
                (None, None) => break
            }
        }

        #[cfg(feature="stats_events")]
        for stat in self.stats.iter() {
            self.stats_events[&stat.0].send(stat.1);
        }
    }

    fn clear_entities_per_mask_index(&mut self){
        unsafe{
            let _guard = self.entities_index_per_mask_guard.write().unwrap();
            (*self.entities_index_per_mask.get()).clear();
        }
    }

    pub(crate) fn entities_ref(&self) -> &[(Entity, ::MaskType)]{
        &self.entities
    }

    pub(crate) fn next_guid(&mut self) -> usize{
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn last_guid(&self) -> usize{
        self.next_guid.load(Ordering::SeqCst)
    }

    pub(crate) fn push_entity(&mut self, e: ::Entity, mask: ::MaskType){
        self.clear_entities_per_mask_index();
        self.entities.push((e, mask));
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

    pub(crate) fn components_mask<C: ::Component>(&self) -> MaskType{
        self.components_mask_index.get(&TypeId::of::<C>())
            .expect(&format!("Trying to use component {} before registering", C::type_name()))
            .clone()
    }

    pub(crate) fn entities_for_mask(&self, mask: Bitmask) -> IndexGuard{
        let contains_key = unsafe {
            let _guard = self.entities_index_per_mask_guard.read().unwrap();
            (*self.entities_index_per_mask.get()).contains_key(&mask)
        };
        if !contains_key {
            let entities = self.entities.iter().filter_map(|&(e, ref entity_mask)| {
                    if mask.check(entity_mask.clone()) {
                        Some(e.guid())
                    }else{
                        None
                    }
                }).collect::<Vec<_>>();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask.clone(), RwLock::new(entities));
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

    // This uses the same index as the non ordered which is probably not correct?
    pub(crate) fn ordered_entities_for<'a, C: Component>(&self, mask: Bitmask) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<'a,C>
    {
        if !self.ordered_entities_index_per_mask.write()
            .unwrap()
            .entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){
            let entities = self.storage::<C>()
                    .expect(&format!("Trying to use non registered type {}", C::type_name()))
                    .ordered_ids()
                    .into_iter()
                    .map(|i| *i)
                    .filter(|i| mask.check(self.entities[*i].1.clone()))
                    .collect::<Vec<_>>();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask.clone(), RwLock::new(entities));
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

    pub(crate) fn thread_local_ordered_entities_for<'a, C: Component>(&self, mask: Bitmask) -> IndexGuard
        where <C as Component>::Storage: ::HierarchicalStorage<'a,C>
    {
        if !self.ordered_entities_index_per_mask.write()
            .unwrap()
            .entry(TypeId::of::<<C as ::Component>::Storage>())
            .or_insert_with(|| HashMap::new())
            .contains_key(&mask){

            let entities = self.storage::<C>()
                    .expect(&format!("Trying to use non registered type {}", C::type_name()))
                    .ordered_ids()
                    .into_iter()
                    .map(|i| *i)
                    .filter(|i| mask.check(self.entities[*i].1.clone()) )
                    .collect::<Vec<_>>();
            unsafe{
                let _guard = self.entities_index_per_mask_guard.write().unwrap();
                (*self.entities_index_per_mask.get()).insert(mask.clone(), RwLock::new(entities));
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
