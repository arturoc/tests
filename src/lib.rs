#![cfg_attr(feature = "unstable", feature(core_intrinsics))]
#![cfg_attr(feature = "unstable", feature(test))]
#![feature(conservative_impl_trait)]

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::cell::{Ref, RefMut, RefCell};
use std::marker;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::mem;


#[cfg(feature="unstable")]
mod benches;

pub struct World{
    storages: HashMap<TypeId, Box<Any>>,
    next_guid: AtomicUsize,
    entities: Vec<Entity>,
    next_component_mask: AtomicUsize,
    components_mask_index: HashMap<TypeId, usize>,
}

impl World{
    pub fn new() -> World{
        World{
            storages: HashMap::new(),
            next_guid: AtomicUsize::new(0),
            next_component_mask: AtomicUsize::new(1),
            entities: Vec::new(),
            components_mask_index: HashMap::new(),
        }
    }

    pub fn register<C: Component>(&mut self) {
        let type_id = TypeId::of::<C>();
        let storage = Box::new(RefCell::new(<C as Component>::Storage::new())) as Box<Any>;
        let next_mask_mut = self.next_component_mask.get_mut();
        let next_mask = *next_mask_mut;
        *next_mask_mut *= 2;
        self.components_mask_index.insert(type_id, next_mask);
        self.storages.insert(type_id, storage);
    }

    pub fn create_entity(&mut self) -> EntityBuilder{
        EntityBuilder::new(self)
    }

    pub fn iter_for<'a, U: UnorderedData<'a>>(&'a self) -> <U as UnorderedData>::Iter{
        U::into_iter(self)
    }

    pub(crate) fn iter<C: Component>(&self) -> <Ref<<C as Component>::Storage> as IntoIter>::Iter
        where for<'a> Ref<'a, <C as Component>::Storage>: IntoIter
    {
        let type_id = TypeId::of::<C>();
        let storage = &self.storages[&type_id];
        let storage: &RefCell<<C as Component>::Storage> = storage.downcast_ref().unwrap();
        storage.borrow().iter()
    }

    pub(crate) fn iter_mut<C: Component>(&self) -> <RefMut<<C as Component>::Storage> as IntoIterMut>::IterMut
        where for<'a> RefMut<'a, <C as Component>::Storage>: IntoIterMut
    {
        let type_id = TypeId::of::<C>();
        let storage = &self.storages[&type_id];
        let storage: &RefCell<<C as Component>::Storage> = storage.downcast_ref().unwrap();
        storage.borrow_mut().iter_mut()
    }

    pub(crate) fn next_guid(&mut self) -> usize{
        self.next_guid.fetch_add(1, Ordering::SeqCst)
    }

    pub(crate) fn push_entity(&mut self, e: ::Entity){
        self.entities.push(e)
    }

    pub(crate) fn storage<C: ::Component>(&self) -> Option<Ref<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.borrow()
        })
    }

    pub(crate) fn storage_mut<C: ::Component>(&self) -> Option<RefMut<<C as ::Component>::Storage>> {
        self.storages.get(&TypeId::of::<C>()).map(|s| {
            let s: &RefCell<<C as ::Component>::Storage> = s.downcast_ref().unwrap();
            s.borrow_mut()
        })
    }

    pub(crate) fn components_mask<C: ::Component>(&self) -> usize{
        self.components_mask_index[&TypeId::of::<C>()]
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

pub trait Component: 'static + Sized
{
    type Storage: Storage<Self>;
}

pub trait Storage<T>{
    fn new() -> Self;
    fn insert(&mut self, guid: usize, t: T);
    unsafe fn get(&self, guid: usize) -> &T;
    unsafe fn get_mut(&mut self, guid: usize) -> &mut T;
}

pub trait IntoIter{
    type Iter: Iterator;
    fn iter(&self) -> Self::Iter;
}

pub trait IntoIterMut{
    type IterMut: Iterator;
    fn iter_mut(&mut self) -> Self::IterMut;
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

impl<'a, T> IntoIter for Ref<'a, DenseVec<T>>{
    type Iter = DenseIter<'a, T>;
    fn iter(&self) -> DenseIter<'a, T>{
        DenseIter{
            ptr: self.storage.as_ptr(),
            end: unsafe{ self.storage.as_ptr().offset(self.storage.len() as isize) },
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterMut for RefMut<'a, DenseVec<T>>{
    type IterMut = DenseIterMut<'a, T>;
    fn iter_mut(&mut self) -> DenseIterMut<'a, T>{
        DenseIterMut{
            ptr: self.storage.as_mut_ptr(),
            end: unsafe{ self.storage.as_mut_ptr().offset(self.storage.len() as isize) },
            _marker: marker::PhantomData,
        }
    }
}

pub struct DenseIter<'a, T: 'a>{
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
    storage: Ref<'a, DenseVec<T>>,
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

pub struct Read<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct Write<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageRead<'a, S: Storage<T> + 'a, T: 'a + Component>{
    storage: Ref<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWrite<'a, S: Storage<T> + 'a, T: 'a + Component>{
    storage: RefCell<RefMut<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

pub trait StorageRef<'a, T>{
    fn get(&self, guid: usize) -> T;
}

impl<'a, S: Storage<T> + 'a, T: 'a + Component> StorageRef<'a, &'a T> for StorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> &'a T{
        unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
    }
}

impl<'a, S: Storage<T> + 'a, T: 'a + Component> StorageRef<'a, &'a mut T> for StorageWrite<'a, S, T>{
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

impl<'a, T: 'a + Component> UnorderedData<'a> for Read<'a,T>
    where for<'b> Ref<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <Ref<'a, <T as Component>::Storage> as IntoIter>::Iter;
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

impl<'a, T: 'a + Component> UnorderedData<'a> for Write<'a,T>
    where for<'b> RefMut<'b, <T as Component>::Storage>: IntoIterMut
{
    type Iter = <RefMut<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
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

pub struct CombinedUnorderedIter<'a,T1, S1:'a,T2, S2: 'a>{
    mask: usize,
    entities: &'a [Entity],
    storage1: S1,
    _marker1: marker::PhantomData<T1>,
    storage2: S2,
    _marker2: marker::PhantomData<T2>,
    next: usize,
}

impl<'a,T1,S1: StorageRef<'a,T1> + 'a,T2,S2: StorageRef<'a,T2> + 'a> Iterator for CombinedUnorderedIter<'a,T1,S1,T2,S2>{
    type Item = (T1,T2);
    fn next(&mut self) -> Option<Self::Item>{
        if self.next == self.entities.len(){
            None
        }else{
            let next = self.next;
            self.next += 1;
            self.entities[next..].iter()
                .find(|e| e.components_mask & self.mask == self.mask)
                .map(|e| (self.storage1.get(e.guid()), self.storage2.get(e.guid())))
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
            mask: Self::components_mask(world),
            entities: &world.entities,
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

#[cfg(test)]
mod tests {
    #[test]
    fn insert_iter() {
        #[derive(Debug,PartialEq,Copy,Clone)]
        struct Pos{
            x: f32,
            y: f32,
        };

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
}
