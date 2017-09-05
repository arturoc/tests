#![cfg_attr(feature = "unstable", feature(core_intrinsics))]
#![cfg_attr(feature = "unstable", feature(test))]
#![feature(conservative_impl_trait)]

extern crate rayon;
// use rayon::prelude::*;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::cell::{RefCell, Ref, RefMut};
use std::marker;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::ops::{Deref, DerefMut};

#[cfg(feature="unstable")]
mod benches;
#[cfg(feature="unstable")]
mod parallel_benches;

pub struct World{
    storages: HashMap<TypeId, Box<Any>>,
    storages_thread_local: HashMap<TypeId, Box<Any>>,
    next_guid: AtomicUsize,
    entities: Vec<Entity>,
    next_component_mask: AtomicUsize,
    components_mask_index: HashMap<TypeId, usize>,

    entities_index_per_mask: RwLock<HashMap<usize, Vec<usize>>>,
}

unsafe impl Send for World{}
unsafe impl Sync for World{}

struct IndexGuard<'a>{
    _index_guard: RwLockReadGuard<'a, HashMap<usize, Vec<usize>>>,
    index: &'a [usize],
}

pub enum StorageReadGuard<'a, S: 'a>{
    ThreadLocal(Ref<'a,S>),
    Sync(RwLockReadGuard<'a,S>),
}

pub enum StorageWriteGuard<'a, S: 'a>{
    ThreadLocal(RefMut<'a,S>),
    Sync(RwLockWriteGuard<'a,S>),
}

impl<'a, S: 'a> Deref for StorageReadGuard<'a, S>{
    type Target = S;
    fn deref(&self) -> &S{
        match self{
            &StorageReadGuard::ThreadLocal(ref s) => s.deref(),
            &StorageReadGuard::Sync(ref s) => s.deref(),
        }
    }
}

impl<'a, S: 'a> Deref for StorageWriteGuard<'a, S>{
    type Target = S;
    fn deref(&self) -> &S{
        match self{
            &StorageWriteGuard::ThreadLocal(ref s) => s.deref(),
            &StorageWriteGuard::Sync(ref s) => s.deref(),
        }
    }
}

impl<'a, S: 'a> DerefMut for StorageWriteGuard<'a, S>{
    fn deref_mut(&mut self) -> &mut S{
        match self{
            &mut StorageWriteGuard::ThreadLocal(ref mut s) => s.deref_mut(),
            &mut StorageWriteGuard::Sync(ref mut s) => s.deref_mut(),
        }
    }
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
        }
    }

    pub fn register<C: ComponentSync>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RwLock::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask_mut = self.next_component_mask.get_mut();
        let next_mask = *next_mask_mut;
        *next_mask_mut *= 2;
        self.components_mask_index.insert(type_id, next_mask);
        self.storages.insert(type_id, storage);
    }

    pub fn register_thread_local<C: ComponentThreadLocal>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RefCell::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask_mut = self.next_component_mask.get_mut();
        let next_mask = *next_mask_mut;
        *next_mask_mut *= 2;
        self.components_mask_index.insert(type_id, next_mask);
        self.storages_thread_local.insert(type_id, storage);
    }

    pub fn create_entity(&mut self) -> EntityBuilder{
        self.entities_index_per_mask.write().unwrap().clear();
        EntityBuilder::new(self)
    }

    pub fn iter_for<'a, U: UnorderedData<'a>>(&'a self) -> <U as UnorderedData>::Iter{
        U::into_iter(self)
    }

    pub(crate) fn iter<C: Component>(&self) -> <StorageReadGuard<<C as Component>::Storage> as IntoIter>::Iter
        where for<'a> StorageReadGuard<'a, <C as Component>::Storage>: IntoIter
    {
        self.storage_thread_local::<C>().unwrap().iter()
    }

    pub(crate) fn iter_mut<C: Component>(&self) -> <StorageWriteGuard<<C as Component>::Storage> as IntoIterMut>::IterMut
        where for<'a> StorageWriteGuard<'a, <C as Component>::Storage>: IntoIterMut
    {
        self.storage_thread_local_mut::<C>().unwrap().iter_mut()
    }

    pub(crate) fn next_guid(&mut self) -> usize{
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn push_entity(&mut self, e: ::Entity){
        self.entities.push(e)
    }

    pub(crate) fn storage<C: ::Component>(&self) -> Option<StorageReadGuard<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            StorageReadGuard::Sync(s.read().unwrap())
        })
    }

    pub(crate) fn storage_mut<C: ::Component>(&self) -> Option<StorageWriteGuard<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RwLock<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            StorageWriteGuard::Sync(s.write().unwrap())
        })
    }

    pub(crate) fn storage_thread_local<C: ::Component>(&self) -> Option<StorageReadGuard<<C as ::Component>::Storage>> {
        let local = self.storages_thread_local.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            StorageReadGuard::ThreadLocal(s.borrow())
        });
        if local.is_some(){
            local
        }else{
            self.storage::<C>()
        }
    }

    pub(crate) fn storage_thread_local_mut<C: ::Component>(&self) -> Option<StorageWriteGuard<<C as ::Component>::Storage>> {
        let local = self.storages_thread_local.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            StorageWriteGuard::ThreadLocal(s.borrow_mut())
        });
        if local.is_some(){
            local
        }else{
            self.storage_mut::<C>()
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
}

#[derive(Clone,Copy,Eq,PartialEq,Debug)]
pub struct Entity {
    guid: usize,
    pub(crate) components_mask: usize,
}

impl Entity{
    pub fn guid(&self) -> usize{
        self.guid
    }
}

pub struct EntityBuilder<'a>{
    world: &'a mut World,
    guid: usize,
    components_mask: usize,
}

impl<'a> EntityBuilder<'a>{
    pub fn new(world: &'a mut World) -> EntityBuilder{
        let next_guid = world.next_guid();
        EntityBuilder{
            guid: next_guid,
            world: world,
            components_mask: 0,
        }
    }

    pub fn build(&mut self) -> Entity{
        let entity = Entity{
            guid: self.guid,
            components_mask: self.components_mask,
        };
        self.world.push_entity(entity.clone());
        entity
    }

    pub fn add<C: Component>(&mut self, component: C) -> &mut Self {
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", "type_name");//C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }
}

pub trait Component: 'static + Sized {
    type Storage: Storage<Self>;
}

pub trait ComponentSync: Component{}
impl<C: Component + Send> ComponentSync for C{}
pub trait ComponentThreadLocal: Component{}

pub trait Storage<T>{
    fn new() -> Self;
    fn insert(&mut self, guid: usize, t: T);
    unsafe fn get(&self, guid: usize) -> &T;
    unsafe fn get_mut(&mut self, guid: usize) -> &mut T;
}

pub trait IntoIter{
    type Iter: Iterator;
    fn iter(self) -> Self::Iter;
}

pub trait IntoIterMut{
    type IterMut: Iterator;
    fn iter_mut(self) -> Self::IterMut;
}

pub struct DenseVec<T>{
    storage: Vec<T>,
    index: Vec<usize>,
}

impl<T> Storage<T> for DenseVec<T>{
    fn new() -> DenseVec<T>{
        DenseVec{
            storage: vec![],
            index: vec![],
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        let id = self.storage.len();
        self.storage.push(t);
        if self.index.capacity() < guid + 1{
            let diff = guid + 1 - self.index.len();
            self.index.reserve(diff);
        }
        if self.index.len() < guid +1 {
            unsafe{ self.index.set_len(guid+1) }
        }
        unsafe{ ptr::write(self.index.get_unchecked_mut(guid), id) };
    }

    unsafe fn get(&self, guid: usize) -> &T{
        self.storage.get_unchecked(self.index[guid])
    }

    unsafe fn get_mut(&mut self, guid: usize) -> &mut T{
        self.storage.get_unchecked_mut(self.index[guid])
    }
}

impl<'a, T> IntoIter for StorageReadGuard<'a, DenseVec<T>>{
    type Iter = DenseIter<'a, T>;
    fn iter(self) -> DenseIter<'a, T>{
        DenseIter{
            ptr: self.storage.as_ptr(),
            end: unsafe{ self.storage.as_ptr().offset(self.storage.len() as isize) },
            _guard: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterMut for StorageWriteGuard<'a, DenseVec<T>>{
    type IterMut = DenseIterMut<'a, T>;
    fn iter_mut(mut self) -> DenseIterMut<'a, T>{
        DenseIterMut{
            ptr: self.storage.as_mut_ptr(),
            end: unsafe{ self.storage.as_mut_ptr().offset(self.storage.len() as isize) },
            _guard: self,
            _marker: marker::PhantomData,
        }
    }
}

pub struct DenseIter<'a, T: 'a>{
    _guard: StorageReadGuard<'a, DenseVec<T>>,
    ptr: *const T,
    end: *const T,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for DenseIter<'a, T>{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T>{
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                let ret = Some(&*self.ptr);
                self.ptr = self.ptr.offset(1);
                ret
            }
        }
    }
}

pub struct DenseIterMut<'a, T: 'a>{
    _guard: StorageWriteGuard<'a, DenseVec<T>>,
    ptr: *mut T,
    end: *mut T,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for DenseIterMut<'a, T>{
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T>{
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                let ret = Some(&mut *self.ptr);
                self.ptr = self.ptr.offset(1);
                ret
            }
        }
    }
}

pub struct DenseUnorderedIter<'a, T: 'a>{
    storage: RwLockReadGuard<'a, DenseVec<T>>,
    ids: &'a [usize],
    next: usize,
}

impl<'a, T: 'a> Iterator for DenseUnorderedIter<'a, T>{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T>{
        if self.next == self.ids.len(){
            None
        }else{
            let ret = Some(unsafe{ mem::transmute::<&T,&T>(self.storage.get(self.ids[self.next])) });
            self.next += 1;
            ret
        }
    }
}


// Sync Read/Write
pub struct Read<'a, T: 'a + ComponentSync>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct Write<'a, T: 'a + ComponentSync>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageRead<'a, S: Storage<T> + 'a, T: 'a + ComponentSync>{
    storage: StorageReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWrite<'a, S: Storage<T> + 'a, T: 'a + ComponentSync>{
    storage: RefCell<StorageWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

pub trait StorageRef<'a, T>{
    fn get(&self, guid: usize) -> T;
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, &'a T> for StorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> &'a T{
        unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
    }
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, &'a mut T> for StorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> &'a mut T{
        unsafe{ mem::transmute::<&mut T, &mut T>(self.storage.borrow_mut().get_mut(guid)) }
    }
}

pub trait UnorderedData<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> usize;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
}

impl<'a, T: 'a + ComponentSync> UnorderedData<'a> for Read<'a,T>
    where for<'b> StorageReadGuard<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <StorageReadGuard<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a T;
    type Storage = StorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.iter::<T>()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T: 'a + ComponentSync> UnorderedData<'a> for Write<'a,T>
    where for<'b> StorageWriteGuard<'b, <T as Component>::Storage>: IntoIterMut
{
    type Iter = <StorageWriteGuard<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = &'a mut T;
    type Storage = StorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.iter_mut::<T>()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageWrite{
            storage: RefCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
}


// Thread local Read/Write
pub struct ReadLocal<'a, T: 'a + ComponentThreadLocal>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteLocal<'a, T: 'a + ComponentThreadLocal>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageReadLocal<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: StorageReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWriteLocal<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: RefCell<StorageWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

pub trait StorageLocalRef<'a, T>{
    fn get(&self, guid: usize) -> T;
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal> StorageLocalRef<'a, &'a T> for StorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a T{
        unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
    }
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal> StorageLocalRef<'a, &'a mut T> for StorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a mut T{
        unsafe{ mem::transmute::<&mut T, &mut T>(self.storage.borrow_mut().get_mut(guid)) }
    }
}

pub trait UnorderedDataLocal<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> usize;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
}

impl<'a, T: 'a + ComponentThreadLocal> UnorderedDataLocal<'a> for ReadLocal<'a,T>
    where for<'b> StorageReadGuard<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <StorageReadGuard<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a T;
    type Storage = StorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.iter::<T>()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageReadLocal{
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T: 'a + ComponentThreadLocal> UnorderedDataLocal<'a> for WriteLocal<'a,T>
    where for<'b> StorageWriteGuard<'b, <T as Component>::Storage>: IntoIterMut
{
    type Iter = <StorageWriteGuard<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = &'a mut T;
    type Storage = StorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.iter_mut::<T>()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageWriteLocal{
            storage: RefCell::new(world.storage_thread_local_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
}



//-------------------------------------------------------------------
// Combined iterators, generalize for more than 2
pub struct CombinedUnorderedIter<'a,T1, S1:'a,T2, S2: 'a>{
    //mask: usize,
    //entities: &'a [Entity],
    ids: IndexGuard<'a>,
    storage1: S1,
    _marker1: marker::PhantomData<T1>,
    storage2: S2,
    _marker2: marker::PhantomData<T2>,
    next: usize,
}

impl<'a,T1,S1: StorageRef<'a,T1> + 'a,T2,S2: StorageRef<'a,T2> + 'a> Iterator for CombinedUnorderedIter<'a,T1,S1,T2,S2>{
    type Item = (T1,T2);
    fn next(&mut self) -> Option<Self::Item>{
        // if self.next == self.entities.len(){
        //     None
        // }else{
        //     let next = self.next;
        //     self.next += 1;
        //     self.entities[next..].iter()
        //         .find(|e| e.components_mask & self.mask == self.mask)
        //         .map(|e| (self.storage1.get(e.guid()), self.storage2.get(e.guid())))
        // }
        if self.next == self.ids.index.len(){
            None
        }else{
            let guid = self.ids.index[self.next];
            self.next += 1;
            Some((self.storage1.get(guid), self.storage2.get(guid)))
        }
    }
}

pub struct CombinedStorageRef<S1,S2>{
    storage1: S1,
    storage2: S2,
}

impl<'a,T1,T2,S1:StorageRef<'a,T1>,S2:StorageRef<'a,T2>> StorageRef<'a, (T1,T2)> for CombinedStorageRef<S1,S2>{
    fn get(&self, guid: usize) -> (T1,T2){
        (self.storage1.get(guid), self.storage2.get(guid))
    }
}

impl<'a, U1: UnorderedData<'a>, U2: UnorderedData<'a>> UnorderedData<'a> for (U1,U2)
    where <U1 as UnorderedData<'a>>::Storage: 'a,
          <U2 as UnorderedData<'a>>::Storage: 'a,
          U1: 'a,
          U2: 'a,
{
    type Iter = CombinedUnorderedIter<'a,<U1 as UnorderedData<'a>>::ComponentsRef, <U1 as UnorderedData<'a>>::Storage, <U2 as UnorderedData<'a>>::ComponentsRef, <U2 as UnorderedData<'a>>::Storage>;
    type Components = (<U1 as UnorderedData<'a>>::Components, <U2 as UnorderedData<'a>>::Components);
    type ComponentsRef = (<U1 as UnorderedData<'a>>::ComponentsRef, <U2 as UnorderedData<'a>>::ComponentsRef);
    type Storage = CombinedStorageRef<<U1 as UnorderedData<'a>>::Storage, <U2 as UnorderedData<'a>>::Storage>;
    fn components_mask(world: &'a World) -> usize{
        U1::components_mask(world) | U2::components_mask(world)
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        CombinedUnorderedIter{
            // mask: Self::components_mask(world),
            // entities: &world.entities,
            ids: world.entities_for_mask(Self::components_mask(world)),
            storage1: U1::storage(world),
            _marker1: marker::PhantomData,
            storage2: U2::storage(world),
            _marker2: marker::PhantomData,
            next: 0,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        CombinedStorageRef{
            storage1: U1::storage(world),
            storage2: U2::storage(world),
        }
    }
}











pub struct CombinedUnorderedIter3<'a,T1, S1:'a,T2, S2: 'a,T3,S3: 'a>{
    //mask: usize,
    //entities: &'a [Entity],
    ids: IndexGuard<'a>,
    storage1: S1,
    _marker1: marker::PhantomData<T1>,
    storage2: S2,
    _marker2: marker::PhantomData<T2>,
    storage3: S3,
    _marker3: marker::PhantomData<T3>,
    next: usize,
}

impl<'a,T1,S1: StorageRef<'a,T1> + 'a,T2,S2: StorageRef<'a,T2> + 'a,T3,S3: StorageRef<'a,T3> + 'a> Iterator for CombinedUnorderedIter3<'a,T1,S1,T2,S2,T3,S3>{
    type Item = (T1,T2,T3);
    fn next(&mut self) -> Option<Self::Item>{
        // if self.next == self.entities.len(){
        //     None
        // }else{
        //     let next = self.next;
        //     self.next += 1;
        //     self.entities[next..].iter()
        //         .find(|e| e.components_mask & self.mask == self.mask)
        //         .map(|e| (self.storage1.get(e.guid()), self.storage2.get(e.guid())))
        // }
        if self.next == self.ids.index.len(){
            None
        }else{
            let guid = self.ids.index[self.next];
            self.next += 1;
            Some((self.storage1.get(guid), self.storage2.get(guid), self.storage3.get(guid)))
        }
    }
}

pub struct CombinedStorageRef3<S1,S2,S3>{
    storage1: S1,
    storage2: S2,
    storage3: S3,
}

impl<'a,T1,T2,T3,S1:StorageRef<'a,T1>,S2:StorageRef<'a,T2>,S3:StorageRef<'a,T3>> StorageRef<'a, (T1,T2,T3)> for CombinedStorageRef3<S1,S2,S3>{
    fn get(&self, guid: usize) -> (T1,T2,T3){
        (self.storage1.get(guid), self.storage2.get(guid), self.storage3.get(guid))
    }
}

impl<'a, U1: UnorderedData<'a>, U2: UnorderedData<'a>, U3: UnorderedData<'a>> UnorderedData<'a> for (U1,U2,U3)
    where <U1 as UnorderedData<'a>>::Storage: 'a,
          <U2 as UnorderedData<'a>>::Storage: 'a,
          <U3 as UnorderedData<'a>>::Storage: 'a,
          U1: 'a,
          U2: 'a,
          U3: 'a,
{
    type Iter = CombinedUnorderedIter3<'a,
                    <U1 as UnorderedData<'a>>::ComponentsRef, <U1 as UnorderedData<'a>>::Storage,
                    <U2 as UnorderedData<'a>>::ComponentsRef, <U2 as UnorderedData<'a>>::Storage,
                    <U3 as UnorderedData<'a>>::ComponentsRef, <U3 as UnorderedData<'a>>::Storage>;
    type Components = (<U1 as UnorderedData<'a>>::Components,
                       <U2 as UnorderedData<'a>>::Components,
                       <U3 as UnorderedData<'a>>::Components);
    type ComponentsRef = (<U1 as UnorderedData<'a>>::ComponentsRef,
                          <U2 as UnorderedData<'a>>::ComponentsRef,
                          <U3 as UnorderedData<'a>>::ComponentsRef);
    type Storage = CombinedStorageRef3<
                        <U1 as UnorderedData<'a>>::Storage,
                        <U2 as UnorderedData<'a>>::Storage,
                        <U3 as UnorderedData<'a>>::Storage>;
    fn components_mask(world: &'a World) -> usize{
        U1::components_mask(world) | U2::components_mask(world) | U3::components_mask(world)
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        CombinedUnorderedIter3{
            // mask: Self::components_mask(world),
            // entities: &world.entities,
            ids: world.entities_for_mask(Self::components_mask(world)),
            storage1: U1::storage(world),
            _marker1: marker::PhantomData,
            storage2: U2::storage(world),
            _marker2: marker::PhantomData,
            storage3: U3::storage(world),
            _marker3: marker::PhantomData,
            next: 0,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        CombinedStorageRef3{
            storage1: U1::storage(world),
            storage2: U2::storage(world),
            storage3: U3::storage(world),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn insert_iter() {
        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Pos{
            x: f32,
            y: f32,
        }

        impl ::Component for Pos{
            type Storage = ::DenseVec<Pos>;
        }

        let mut world = ::World::new();
        world.register::<Pos>();
        world.create_entity()
            .add(Pos{x: 1., y: 1.})
            .build();
        world.create_entity()
            .add(Pos{x: 2., y: 2.})
            .build();
        world.create_entity()
            .add(Pos{x: 3., y: 3.})
            .build();

        assert_eq!(world.iter::<Pos>().count(), 3);
        let mut iter = world.iter::<Pos>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn insert_read() {
        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Pos{
            x: f32,
            y: f32,
        }

        impl ::Component for Pos{
            type Storage = ::DenseVec<Pos>;
        }

        let mut world = ::World::new();
        world.register::<Pos>();
        world.create_entity()
            .add(Pos{x: 1., y: 1.})
            .build();
        world.create_entity()
            .add(Pos{x: 2., y: 2.})
            .build();
        world.create_entity()
            .add(Pos{x: 3., y: 3.})
            .build();

        assert_eq!(world.iter_for::<::Read<Pos>>().count(), 3);
        let mut iter = world.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn insert_read_write() {
        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Pos{
            x: f32,
            y: f32,
        }

        impl ::Component for Pos{
            type Storage = ::DenseVec<Pos>;
        }

        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Vel{
            x: f32,
            y: f32,
        }

        impl ::Component for Vel{
            type Storage = ::DenseVec<Vel>;
        }

        let mut world = ::World::new();
        world.register::<Pos>();
        world.register::<Vel>();
        world.create_entity()
            .add(Pos{x: 1., y: 1.})
            .add(Vel{x: 1., y: 1.})
            .build();
        world.create_entity()
            .add(Pos{x: 2., y: 2.})
            .add(Vel{x: 1., y: 1.})
            .build();
        world.create_entity()
            .add(Pos{x: 3., y: 3.})
            .add(Vel{x: 1., y: 1.})
            .build();

        for (pos, vel) in world.iter_for::<(::Write<Pos>, ::Read<Vel>)>(){
            pos.x += vel.x;
            pos.y += vel.y;
        }

        assert_eq!(world.iter_for::<::Read<Pos>>().count(), 3);
        let mut iter = world.iter_for::<::Read<Pos>>();
        assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
        assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
        assert_eq!(iter.next(), Some(&Pos{x: 4., y: 4.}));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn insert_read_write_parallel() {
        use rayon;
        use rayon::prelude::*;

        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Pos{
            x: f32,
            y: f32,
        }

        impl ::Component for Pos{
            type Storage = ::DenseVec<Pos>;
        }

        struct C1;
        impl ::Component for C1{
            type Storage = ::DenseVec<C1>;
        }

        struct C2;
        impl ::Component for C2{
            type Storage = ::DenseVec<C2>;
        }

        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Vel{
            x: f32,
            y: f32,
        }

        impl ::Component for Vel{
            type Storage = ::DenseVec<Vel>;
        }

        let mut world = ::World::new();

        world.register::<Pos>();
        world.register::<Vel>();
        world.register::<C1>();
        world.register::<C2>();

        for i in 0..100usize{
            world.create_entity()
                .add(Pos{x: i as f32, y: i as f32})
                .add(C1)
                .add(Vel{x: 1., y: 1.})
                .build();
        }

        for i in 0..100usize{
            world.create_entity()
                .add(Pos{x: i as f32, y: i as f32})
                .add(C2)
                .add(Vel{x: 1., y: 1.})
                .build();
        }

        fn write1(w: &::World){
            for (pos, _, vel) in w.iter_for::<(::Write<Pos>, ::Read<C1>, ::Read<Vel>)>(){
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        fn write2(w: &::World){
            for (pos, _, vel) in w.iter_for::<(::Write<Pos>, ::Read<C2>, ::Read<Vel>)>(){
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        rayon::join(|| write1(&world), || write2(&world));

        assert_eq!(world.iter_for::<::Read<Pos>>().count(), 200);
        let mut iter = world.iter_for::<::Read<Pos>>();
        for i in 0..100{
            assert_eq!(iter.next(), Some(&Pos{x: (i + 1) as f32, y: (i + 1) as f32}));
        }
        for i in 0..100{
            assert_eq!(iter.next(), Some(&Pos{x: (i + 1) as f32, y: (i + 1) as f32}));
        }
    }

    // #[test]
    // fn parallel_build(){
    //     use std::thread;
    //     use rayon::prelude::*;
    //
    //     #[derive(Debug,PartialEq,Copy,Clone)]
    //     struct Pos{
    //         x: f32,
    //         y: f32,
    //     }
    //
    //     impl ::Component for Pos{
    //         type Storage = ::DenseVec<Pos>;
    //     }
    //
    //     let mut world = ::World::new();
    //     world.register::<Pos>();
    //
    //     // Won't work since we hold world mutably
    //     vec![0,1,2,3,4,5,6,7,8,9].par_iter().for_each(|i|{
    //         world.create_entity()
    //             .add(Pos{x: *i as f32, y: *i as f32})
    //             .build();
    //     });
    //
    //     assert_eq!(world.iter_for::<::Read<Pos>>().count(), 3);
    //     let mut iter = world.iter_for::<::Read<Pos>>();
    //     assert_eq!(iter.next(), Some(&Pos{x: 1., y: 1.}));
    //     assert_eq!(iter.next(), Some(&Pos{x: 2., y: 2.}));
    //     assert_eq!(iter.next(), Some(&Pos{x: 3., y: 3.}));
    //     assert_eq!(iter.next(), None);
    // }
}
