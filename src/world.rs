use std::any::{Any, TypeId};
// use std::collections::HashMap;
use fxhash::FxHashMap as HashMap;
use std::cell::{RefCell, Ref, RefMut, UnsafeCell};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::mem;
use ::System;
use ::SystemThreadLocal;
use smallvec;

use ::Entity;
use component::{self, ComponentSync, Component, ComponentThreadLocal,
    OneToNComponentSync, OneToNComponentThreadLocal};
use storage::{Storage, HierarchicalStorage, OneToNStorage};
use entity::{EntityBuilder, Entities, EntitiesThreadLocal};
use sync::*;
use rayon::prelude::*;
use ::{Bitmask, MaskType, NextMask};
#[cfg(feature="dynamic_systems")]
use dynamic_system_loader::DynamicSystemsLoader;

#[cfg(feature="stats_events")]
use seitan::*;
#[cfg(feature="stats_events")]
use std::time;

#[derive(Clone, Copy, Debug)]
enum Priority{
    Send(usize),
    ThreadLocal(usize),
    Creation(usize),
    Barrier
}

pub struct World{
    storages: HashMap<component::Id, Box<Any>>,
    storages_thread_local: HashMap<component::Id, Box<Any>>,
    resources: HashMap<TypeId, Box<Any>>,

    next_guid: AtomicUsize,
    entities: Vec<(Entity, ::MaskType)>, // Doesn't need lock cause never accesed mut from Entities?
    entities_index_per_mask: UnsafeCell<HashMap<Bitmask, RwLock<Vec<usize>>>>,
    entities_index_per_mask_guard: RwLock<()>,
    ordered_entities_index_per_mask: RwLock<HashMap<component::Id, HashMap<Bitmask, Vec<usize>>>>,
    reverse_components_mask_index: HashMap<MaskType, component::Id>,
    remove_components_mask_index: HashMap<MaskType, Box<Fn(&World, usize)>>,

    next_component_mask: NextMask,
    pub(crate) components_mask_index: HashMap<component::Id, MaskType>,


    systems: Vec<(String, SyncSystem)>,
    systems_thread_local: Vec<(String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
    world_systems: Vec<(String, Box<for<'a> ::CreationSystem<'a>>)>,
    priority_queue: Vec<Priority>,
    // barriers: Vec<(usize)>,>
    // next_system_priority: AtomicUsize,

    #[cfg(feature="stats_events")]
    stats: Vec<(String, time::Duration)>,

    #[cfg(feature="stats_events")]
    stats_events: HashMap<String, SenderRc<'static, time::Duration>>,

    #[cfg(feature="dynamic_systems")]
    dynamic_systems: DynamicSystemsLoader,
}

unsafe impl Send for World{}

impl World{
    pub fn new() -> World{
        World{
            storages: HashMap::default(),
            storages_thread_local: HashMap::default(),

            resources: HashMap::default(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: NextMask::new(),
            entities: Vec::new(),
            components_mask_index: HashMap::default(),
            entities_index_per_mask_guard: RwLock::new(()),
            entities_index_per_mask: UnsafeCell::new(HashMap::default()),
            ordered_entities_index_per_mask: RwLock::new(HashMap::default()),
            reverse_components_mask_index: HashMap::default(),
            remove_components_mask_index: HashMap::default(),
            systems: vec![],
            systems_thread_local: vec![],
            world_systems: vec![],
            // barriers: vec![],
            // next_system_priority: AtomicUsize::new(0),
            priority_queue: vec![],

            #[cfg(feature="stats_events")]
            stats: Vec::new(),

            #[cfg(feature="stats_events")]
            stats_events: HashMap::default(),

            #[cfg(feature="dynamic_systems")]
            dynamic_systems: DynamicSystemsLoader::new().unwrap(),
        }
    }

    pub fn register<C: ComponentSync>(&mut self) {
        if self.storages.get(&C::id()).is_some(){
            panic!("{} already registered or not unique component id", C::type_name());
        }
        if self.storages_thread_local.get(&C::id()).is_some(){
            panic!("{} already registered or not unique component id", C::type_name());
        }
        let storage = Box::new(RwLock::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask = self.next_component_mask.next();
        self.components_mask_index.insert(C::id(), next_mask.clone());
        self.reverse_components_mask_index.insert(next_mask.clone(), C::id());
        self.storages.insert(C::id(), storage);
        self.remove_components_mask_index.insert(next_mask, Box::new(move |world, guid|{
            // let s: &RwLock<<C as ::Component>::Storage> = any.downcast_ref().unwrap();
            // s.write().unwrap().remove(guid)

            world.storage_mut::<C>()
                .expect(&format!("Trying to delete component {} without registering first", C::type_name()))
                .remove(guid);
        }));
    }

    pub fn register_thread_local<C: ComponentThreadLocal>(&mut self) {
        let storage = Box::new(RefCell::new(<C as Component>::Storage::new())) as Box<Any>;
        if self.storages.get(&C::id()).is_some(){
            panic!("{} already registered or not unique component id", C::type_name());
        }
        if self.storages_thread_local.get(&C::id()).is_some(){
            panic!("{} already registered or not unique component id", C::type_name());
        }
        let next_mask = self.next_component_mask.next();
        self.components_mask_index.insert(C::id(), next_mask.clone());
        self.reverse_components_mask_index.insert(next_mask.clone(), C::id());
        self.storages_thread_local.insert(C::id(), storage);
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
        *mask |= self.components_mask_index[&C::id()].clone();
    }

    pub fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C){
        self.clear_entities_per_mask_index();
        self.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&C::id()].clone();
    }

    pub fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        self.storage_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert_slice(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&C::id()].clone();
    }

    pub fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]){
        self.clear_entities_per_mask_index();
        self.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to add component of type {} without registering first", C::type_name()))
            .insert_slice(entity.guid(), component);
        let &mut (_entity, ref mut mask) = &mut self.entities[entity.guid()];
        *mask |= self.components_mask_index[&C::id()].clone();
    }

    pub fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        self.storage_mut::<C>()
            .expect(&format!("Trying to remove component of type {} without registering first", C::type_name()))
            .remove(entity.guid());
        self.entities[entity.guid()].1 ^= self.components_mask_index[&C::id()].clone();
        let mask = self.components_mask::<C>();
        {
            let type_id = &self.reverse_components_mask_index[&mask];
            if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(type_id){
                cache.clear();
            }
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
                let type_id = &self.reverse_components_mask_index[&mask];
                if let Some(cache) = self.ordered_entities_index_per_mask.write().unwrap().get_mut(type_id){
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
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.priority_queue.push(Priority::Send(self.systems.len()));
        self.systems.push((String::new(), SyncSystem::new(system)));
        self
    }

    pub fn add_system_with_data<S,D>(&mut self, system: S, data: D) -> &mut World
    // where  for<'a> S: ::SystemWithData<'a> + 'static
    where S: FnMut(&mut D, Entities, ::Resources) + Send + 'static,
          D: Send + 'static
    {
        self.add_system((system, data))
    }

    pub fn add_system_thread_local<S>(&mut self, system: S) -> &mut World
    where  for<'a> S: ::SystemThreadLocal<'a> + 'static
    {
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.priority_queue.push(Priority::ThreadLocal(self.systems_thread_local.len()));
        self.systems_thread_local.push((String::new(), Box::new(system)));
        self
    }

    pub fn add_system_with_data_thread_local<S,D>(&mut self, system: S, data: D) -> &mut World
    // where  for<'a> S: ::SystemWithData<'a> + 'static
    where S: FnMut(&mut D, EntitiesThreadLocal, ::ResourcesThreadLocal) + 'static,
          D: 'static
    {
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.priority_queue.push(Priority::ThreadLocal(self.systems_thread_local.len()));
        self.systems_thread_local.push((String::new(), Box::new((system, data))));
        self
    }

    pub fn add_creation_system<S>(&mut self, system: S) -> &mut World
    where S: for<'a> ::CreationSystem<'a> + 'static
    {
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.priority_queue.push(Priority::Creation(self.world_systems.len()));
        self.world_systems.push((String::new(), Box::new(system)));
        self
    }

    pub fn add_creation_system_with_data<S, D>(&mut self, system: S, data: D) -> &mut World
    where S: FnMut(&mut D, ::EntitiesCreation, ::ResourcesThreadLocal) + 'static,
          D: 'static
    {
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        self.priority_queue.push(Priority::Creation(self.world_systems.len()));
        self.world_systems.push((String::new(), Box::new((system, data))));
        self
    }

    #[cfg(feature="stats_events")]
    pub fn add_system_with_stats<S>(&mut self, system: S, name: &str) -> &mut World
    where  for<'a> S: ::System<'a> + 'static
    {
        self.add_system(system);
        self.stats_events.insert(name.to_owned(), SenderRc::new());
        self
    }

    #[cfg(feature="stats_events")]
    pub fn add_system_with_stats_thread_local<S>(&mut self, system: S, name: &str) -> &mut World
    where  for<'a> S: ::SystemThreadLocal<'a> + 'static
    {
        self.add_system_thread_local(system);
        self.stats_events.insert(name.to_owned(), SenderRc::new());
        self
    }

    #[cfg(feature="dynamic_systems")]
    pub fn preload_dynamic_libraries(&mut self, libs: &[&str]) -> Result<(), String> {
        self.dynamic_systems.preload_libraries(libs)
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system(&mut self, system_path: &str) -> &mut World{
        let system = self.dynamic_systems.new_system(system_path).unwrap();
        self.add_system(system)
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system_with_data<D: Send + 'static>(&mut self, system_path: &str, data: D) -> &mut World{
        let system = self.dynamic_systems.new_system_with_data(system_path).unwrap();
        self.add_system((system, data))
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system_thread_local(&mut self, system_path: &str) -> &mut World{
        let system = self.dynamic_systems.new_system_thread_local(system_path).unwrap();
        self.add_system_thread_local(system)
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system_with_data_thread_local<D: 'static>(&mut self, system_path: &str, data: D) -> &mut World{
        let system = self.dynamic_systems.new_system_with_data_thread_local(system_path).unwrap();
        self.add_system_thread_local((system, data))
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_creation_system(&mut self, system_path: &str) -> &mut World{
        let system = self.dynamic_systems.new_creation_system(system_path).unwrap();
        self.add_creation_system(system)
    }

    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_creation_system_with_data<D: 'static>(&mut self, system_path: &str, data: D) -> &mut World{
        let system = self.dynamic_systems.new_creation_system_with_data(system_path).unwrap();
        self.add_creation_system((system, data))
    }

    #[cfg(feature="dynamic_systems")]
    pub fn run_dynamic_system_once(&mut self, system_path: &str) -> &mut World{
        let mut system = self.dynamic_systems.new_system(system_path).unwrap();
        system.run(self.entities(), self.resources());
        self
    }

    #[cfg(feature="dynamic_systems")]
    pub fn run_dynamic_system_once_thread_local(&mut self, system_path: &str) -> &mut World{
        let mut system = self.dynamic_systems.new_system_thread_local(system_path).unwrap();
        system.run(self.entities_thread_local(), self.resources_thread_local());
        self
    }

    #[cfg(feature="stats_events")]
    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system_with_stats(&mut self, system_path: &str, name: &str) -> &mut World{
        let system = self.dynamic_systems.new_system(system_path).unwrap();
        self.add_system_with_stats(system, name)
    }

    #[cfg(feature="stats_events")]
    #[cfg(feature="dynamic_systems")]
    pub fn new_dynamic_system_with_stats_thread_local(&mut self, system_path: &str, name: &str) -> &mut World{
        let system = self.dynamic_systems.new_system_thread_local(system_path).unwrap();
        self.add_system_with_stats_thread_local(system, name)
    }

    #[cfg(feature="stats_events")]
    pub fn stats(&mut self) -> Vec<(String, Property<'static, time::Duration>)>{
        self.stats_events.iter_mut()
            .map(|(name, sender)| (name.clone(), sender.stream().to_property(time::Duration::new(0, 0))))
            .collect()
    }

    pub fn add_barrier(&mut self) -> &mut World
    {
        // let prio = self.next_system_priority.fetch_add(1, Ordering::SeqCst);
        // self.barriers.push(prio);
        self.priority_queue.push(Priority::Barrier);
        self
    }

    #[cfg(feature="dynamic_systems")]
    pub fn start_dynamic_systems_watch(&mut self) -> Result<(), String>{
        self.dynamic_systems.start()
    }

    pub fn run_once(&mut self){
        let systems_thread_local = unsafe{ mem::transmute::<
                &mut Vec<(String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>,
                &mut Vec<(String, Box<for<'a> ::system::SystemThreadLocal<'a>>)>
            >(&mut self.systems_thread_local) };
        let world_systems = unsafe{ mem::transmute::<
                &mut Vec<(String, Box<for<'a> ::CreationSystem<'a>>)>,
                &mut Vec<(String, Box<for<'a> ::CreationSystem<'a>>)>
            >(&mut self.world_systems) };


        #[cfg(feature="stats_events")]
        let stats = {
            self.stats.reserve(self.systems.len() + self.systems_thread_local.len());
            self.stats.clear();
            unsafe{ mem::transmute::<
                    &mut Vec<(String, time::Duration)>,
                    &mut Vec<(String, time::Duration)>
                >(&mut self.stats)}
        };
        let world = unsafe{
            mem::transmute::<&mut World, &mut World>(self)
        };
        let mut priority = self.priority_queue.iter().peekable();
        let mut send_systems: smallvec::SmallVec<[&(String, SyncSystem); 128]> = smallvec::SmallVec::new();

        while let Some(next) = priority.next() {
            send_systems.clear();
            match next {
                Priority::Send(i) => {
                    let entities = self.entities();
                    let resources = self.resources();

                    send_systems.push(&self.systems[*i]);
                    while let Some(Priority::Send(i)) = priority.peek(){
                        send_systems.push(&self.systems[*i]);
                        priority.next();
                    }

                    #[cfg(feature="stats_events")]
                    stats.par_extend(send_systems.par_iter().filter_map(|&(ref name, s)| {
                        if name != "" {
                            let then = time::Instant::now();
                            s.borrow_mut().run(entities, resources);
                            let now = time::Instant::now();
                            Some((name.clone(), now - then))
                        }else{
                            s.borrow_mut().run(entities, resources);
                            None
                        }
                    }));

                    #[cfg(not(feature="stats_events"))]
                    send_systems.par_iter().for_each(|&( _, s)| {
                        s.borrow_mut().run(entities, resources)
                    });
                }

                Priority::ThreadLocal(i) => {
                    let (_name, system_tl) = &mut systems_thread_local[*i];
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
                }

                Priority::Creation(i) => {
                    let (_name, system_w) = &mut world_systems[*i];
                    #[cfg(feature="stats_events")]
                    {
                        if _name != "" {
                            let then = time::Instant::now();
                            system_w.run(::EntitiesCreation::new(world), self.resources_thread_local());
                            let now = time::Instant::now();
                            stats.push((_name.clone(), now - then));
                        }else{
                            system_w.run(::EntitiesCreation::new(world), self.resources_thread_local());
                        }
                    }

                    #[cfg(not(feature="stats_events"))]
                    system_w.run(::EntitiesCreation::new(world), self.resources_thread_local());

                }

                Priority::Barrier => ()
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
        self.storages.get(&C::id()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.read().unwrap()
        })
    }

    pub(crate) fn storage_mut<C: ::Component>(&self) -> Option<RwLockWriteGuard<<C as ::Component>::Storage>> {
        self.storages.get(&C::id()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.write().unwrap()
        })
    }

    pub(crate) fn storage_thread_local<C: ::Component>(&self) -> Option<ReadGuardRef<<C as ::Component>::Storage>> {
        let local = self.storages_thread_local.get(&C::id()).map(|s| {
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
        let local = self.storages_thread_local.get(&C::id()).map(|s| {
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
        self.components_mask_index.get(&C::id())
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
            .entry(C::id())
            .or_insert_with(|| HashMap::default())
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
            .entry(C::id())
            .or_insert_with(|| HashMap::default())
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
