use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::cell::UnsafeCell;
use std::mem;
use std::slice;
use std::iter;

use sync::{ReadGuardRef, WriteGuardRef};
use ::Component;
use ::ComponentSync;
use ::ComponentThreadLocal;
use ::World;
use ::IndexGuard;
use ::Entity;
use ::Bitmask;

pub trait Storage<'a, T>{
    type Get;
    type GetMut;
    fn new() -> Self;
    fn with_capacity(usize) -> Self;
    fn insert(&mut self, guid: usize, t: T);
    fn remove(&mut self, guid: usize);
    unsafe fn get(&'a self, guid: usize) -> Self::Get;
    unsafe fn get_mut(&'a mut self, guid: usize) -> Self::GetMut;
    fn contains(&self, guid: usize) -> bool;
}

pub trait IntoIter{
    type Iter: Iterator;
    fn into_iter(self) -> Self::Iter;
}

pub trait IntoIterMut{
    type IterMut: Iterator;
    fn into_iter_mut(self) -> Self::IterMut;
}

pub struct Read<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct Write<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct ReadEntities;

pub struct Not<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct ReadNot<'a, T: 'a + Component, Not: 'a + Component>{
    _marker1: marker::PhantomData<&'a T>,
    _marker2: marker::PhantomData<&'a Not>,
}

// Sync Read/Write
pub struct StorageRead<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWrite<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

pub trait StorageRef<'a, T>{
    fn get(&self, guid: usize) -> T;
    fn contains(&self, guid: usize) -> bool;
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, <S as Storage<'a,T>>::Get> for StorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> <S as Storage<'a,T>>::Get{
        // unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }

        let storage = unsafe{ mem::transmute::<&S, &S>(&self.storage) };
        unsafe{ storage.get(guid) }
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains(guid)
    }
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, <S as Storage<'a,T>>::GetMut> for StorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> <S as Storage<'a,T>>::GetMut{
        // unsafe{ mem::transmute::<&mut T, &mut T>( (*self.storage.get()).get_mut(guid) ) }


        let storage = unsafe{ mem::transmute::<&mut S, &mut S>(&mut (*self.storage.get())) };
        unsafe{ storage.get_mut(guid) }
    }

    fn contains(&self, guid: usize) -> bool {
        unsafe{ (*self.storage.get()).contains(guid) }
    }
}

pub trait UnorderedData<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> Bitmask;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
}

impl<'a, T: 'a + ComponentSync> UnorderedData<'a> for Read<'a,T>
    where for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <RwLockReadGuard<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::Get;
    type Storage = StorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage::<T>().unwrap().into_iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T: 'a + ComponentSync> UnorderedData<'a> for Write<'a,T>
    where for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoIterMut
{
    type Iter = <RwLockWriteGuard<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::GetMut;
    type Storage = StorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_mut::<T>().unwrap().into_iter_mut()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T: 'a + ComponentSync> UnorderedData<'a> for Not<'a,T> {
    type Iter = iter::Repeat<()>;
    type Components = T;
    type ComponentsRef = ();
    type Storage = ();
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::not(world.components_mask::<T>())
    }

    fn into_iter(_: &'a ::World) -> Self::Iter{
        iter::repeat(())
    }

    fn storage(_: &'a ::World) -> (){
        ()
    }
}

impl<'a> StorageRef<'a, ()> for (){
    fn get(&self, _guid: usize) -> (){
        ()
    }

    fn contains(&self, guid: usize) -> bool{
        //TODO: is this correct? surely will never get called
        false
    }
}

pub struct ReadNotIter<'a,T,S>{
    _ids: ::IndexGuard<'a>,
    ptr: *const usize,
    end: *const usize,
    storage: S,
    _marker: marker::PhantomData<T>,
}


impl<'a, T, S: ::StorageRef<'a, T> + 'a> Iterator for ReadNotIter<'a,T,S>{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item>{
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                let guid = *self.ptr;
                self.ptr = self.ptr.offset(1);
                Some(self.storage.get(guid))
            }
        }
    }
}

impl<'a, T: 'a + ComponentSync, Not: Component> UnorderedData<'a> for ReadNot<'a,T,Not> {
    type Iter = ReadNotIter<'a, Self::ComponentsRef, Self::Storage>;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::Get;
    type Storage = StorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask {
        Bitmask::has_not(world.components_mask::<T>(), world.components_mask::<Not>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        let ids = world.entities_for_mask(<Self as UnorderedData>::components_mask(world));
        ReadNotIter{
            ptr: ids.index.as_ptr(),
            end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
            _ids: ids,
            storage: <Self as UnorderedData>::storage(world),
            _marker: marker::PhantomData,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage {
        StorageRead {
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

pub struct ReadOption<'a, T: 'a>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct IterOption<'a, T, S>
        where S: ::Storage<'a, T> + 'a
{
    last_guid: usize,
    next: usize,
    storage: StorageOption<'a, T, S>,
}


impl<'a, T, S: ::StorageRef<'a, T> + 'a> Iterator for IterOption<'a, T,S>
    where S: ::Storage<'a, T> + 'a
{
    type Item = Option<<S as ::Storage<'a,T>>::Get>;
    fn next(&mut self) -> Option<Self::Item>{
        use ::StorageRef;
        unsafe {
            if self.next > self.last_guid {
                None
            } else {
                let next = self.next;
                self.next += 1;
                Some(self.storage.get(next))
            }
        }
    }
}

pub struct StorageOption<'a, T, S>
    where S: 'a
{
    _marker: marker::PhantomData<T>,
    storage: ::ReadGuardRef<'a, S>,
}

impl<'a, T, S> ::StorageRef<'a, Option<<S as ::Storage<'a,T>>::Get>> for StorageOption<'a, T, S>
    where S: ::Storage<'a, T> + 'a
{
    #[allow(non_snake_case)]
    fn get(&self, guid: usize) -> Option<<S as ::Storage<'a,T>>::Get>{
        use boolinator::Boolinator;
        let storage = unsafe{ ::std::mem::transmute::<&S, &S>(&*self.storage) };
        storage.contains(guid)
            .as_some_from(|| unsafe{ storage.get(guid) })
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains(guid)
    }
}

impl<'a, T: 'a + ::ComponentSync> ::UnorderedData<'a> for ::ReadOption<'a,T> {
    type Iter = IterOption<'a, T, <T as ::Component>::Storage>;
    type Components = Option<T>;
    type ComponentsRef = Option<<<T as ::Component>::Storage as ::Storage<'a, T>>::Get>;
    type Storage = StorageOption<'a, T, <T as ::Component>::Storage>;
    fn components_mask(world: &'a ::World) -> ::Bitmask {
        ::Bitmask::all()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        let storage = <Self as ::UnorderedData>::storage(world);
        IterOption {
            next: 0,
            last_guid: world.last_guid(),
            storage,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage {
        StorageOption{
            storage: ::ReadGuardRef::new(::ReadGuard::Sync(world.storage::<T>().unwrap())),
            _marker: marker::PhantomData,
        }
    }
}


impl<'a, T: 'a + ::Component> ::UnorderedDataLocal<'a> for ::ReadOption<'a,T> {
    type Iter = IterOption<'a, T, <T as ::Component>::Storage>;
    type Components = Option<T>;
    type ComponentsRef = Option<<<T as ::Component>::Storage as ::Storage<'a, T>>::Get>;
    type Storage = StorageOption<'a, T, <T as ::Component>::Storage>;
    fn components_mask(world: &'a ::World) -> ::Bitmask {
        ::Bitmask::all()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        let storage = <Self as ::UnorderedDataLocal>::storage(world);
        IterOption {
            next: 0,
            last_guid: world.last_guid(),
            storage: storage,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage {
        StorageOption {
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}



pub struct ReadOr<'a, T: 'a>{
    _marker: marker::PhantomData<&'a T>,
}

//-------------------------------------------------------------------
// ReadOr storage and iterators
macro_rules! impl_or_iter {
    ($iter_or: ident, $storage_or: ident, $($t: ident, $s: ident),*) => (

    pub struct $iter_or<'a,$($t, $s),*>
        where $($s: ::Storage<'a, $t> + 'a),*
    {
        _ids: ::IndexGuard<'a>,
        ptr: *const usize,
        end: *const usize,
        storage: $storage_or<'a, $($t, $s),*>,
    }


    impl<'a, $($t, $s: ::StorageRef<'a, T1> + 'a),*> Iterator for $iter_or<'a,$($t, $s),*>
        where $($s: ::Storage<'a, $t> + 'a),*
    {
        type Item = ($(Option<<$s as ::Storage<'a,$t>>::Get>),*);
        fn next(&mut self) -> Option<Self::Item>{
            use ::StorageRef;
            unsafe {
                if self.ptr == self.end {
                    None
                } else {
                    let guid = *self.ptr;
                    self.ptr = self.ptr.offset(1);
                    Some(self.storage.get(guid))
                }
            }
        }
    }

    #[allow(non_snake_case, dead_code)]
    pub struct $storage_or<'a, $($t, $s),*>
        where $($s: 'a),*
    {
        $(
            $t: marker::PhantomData<$t>,
            $s: ::ReadGuardRef<'a, $s>,
        )*
    }

    impl<'a, $($t, $s),*> ::StorageRef<'a, ($(Option<<$s as ::Storage<'a,$t>>::Get>),*) > for $storage_or<'a, $($t, $s),*>
        where $($s: ::Storage<'a, $t> + 'a),*
    {
        #[allow(non_snake_case)]
        fn get(&self, guid: usize) -> ($(Option<<$s as ::Storage<'a,$t>>::Get>),*){
                use boolinator::Boolinator;
                $(
                    let $s = unsafe{ ::std::mem::transmute::<&$s, &$s>(&*self.$s) };
                    let $t = $s.contains(guid)
                        .as_some_from(|| unsafe{ $s.get(guid) });
                )*
                ($($t),*)
        }

        fn contains(&self, guid: usize) -> bool{
            $(self.$s.contains(guid)) | *
        }
    }

    impl<'a, $($t: 'a + ::ComponentSync),*> ::UnorderedData<'a> for ::ReadOr<'a,($($t),*)> {
        type Iter = $iter_or<'a, $($t, <$t as ::Component>::Storage),*>;
        type Components = ($(Option<$t>),*);
        type ComponentsRef = ($(Option<<<$t as ::Component>::Storage as ::Storage<'a, $t>>::Get>),*);
        type Storage = $storage_or<'a, $($t, <$t as ::Component>::Storage),*>;
        fn components_mask(world: &'a ::World) -> ::Bitmask {
            ::Bitmask::or($(world.components_mask::<$t>()) | *)
        }

        fn into_iter(world: &'a ::World) -> Self::Iter {
            let ids = world.entities_for_mask(<Self as ::UnorderedData>::components_mask(world));
            let storage = <Self as ::UnorderedData>::storage(world);
            $iter_or{
                ptr: ids.index.as_ptr(),
                end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                _ids: ids,
                storage: storage,
            }
        }

        fn storage(world: &'a ::World) -> Self::Storage {
            $storage_or {
                $(
                    $s: ::ReadGuardRef::new(::ReadGuard::Sync(world.storage::<$t>().unwrap())),
                    $t: marker::PhantomData,
                )*
            }
        }
    }


    impl<'a, $($t: 'a + ::Component),*> ::UnorderedDataLocal<'a> for ::ReadOr<'a,($($t),*)> {
        type Iter = $iter_or<'a, $($t, <$t as ::Component>::Storage),*>;
        type Components = ($(Option<$t>),*);
        type ComponentsRef = ($(Option<<<$t as ::Component>::Storage as ::Storage<'a, $t>>::Get>),*);
        type Storage = $storage_or<'a, $($t, <$t as ::Component>::Storage),*>;
        fn components_mask(world: &'a ::World) -> ::Bitmask {
            ::Bitmask::or($( world.components_mask::<$t>() ) | *)
        }

        fn into_iter(world: &'a ::World) -> Self::Iter {
            let ids = world.entities_for_mask(<Self as ::UnorderedDataLocal>::components_mask(world));
            let storage = <Self as ::UnorderedDataLocal>::storage(world);
            $iter_or{
                ptr: ids.index.as_ptr(),
                end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                _ids: ids,
                storage: storage,
            }
        }

        fn storage(world: &'a ::World) -> Self::Storage {
            $storage_or {
                $(
                    $s: world.storage_thread_local::<$t>().unwrap(),
                    $t: marker::PhantomData,
                )*
            }
        }
    }
)}



mod or_storage{
    use std::marker;
    // impl_or_iter!(Iter1, StorageRef1, T1, S1);
    impl_or_iter!(Iter2, StorageRef2, T1, S1, T2, S2);
    impl_or_iter!(Iter3, StorageRef3, T1, S1, T2, S2, T3, S3);
    impl_or_iter!(Iter4, StorageRef4, T1, S1, T2, S2, T3, S3, T4, S4);
    impl_or_iter!(Iter5, StorageRef5, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5);
    impl_or_iter!(Iter6, StorageRef6, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6);
    impl_or_iter!(Iter7, StorageRef7, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7);
    impl_or_iter!(Iter8, StorageRef8, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8);
    impl_or_iter!(Iter9, StorageRef9, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9);
    impl_or_iter!(Iter10, StorageRef10, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10);
    impl_or_iter!(Iter11, StorageRef11, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11);
    impl_or_iter!(Iter12, StorageRef12, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11, T12, S12);
    impl_or_iter!(Iter13, StorageRef13, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11, T12, S12, T13, S13);
    impl_or_iter!(Iter14, StorageRef14, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11, T12, S12, T13, S13, T14, S14);
    impl_or_iter!(Iter15, StorageRef15, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11, T12, S12, T13, S13, T14, S14, T15, S15);
    impl_or_iter!(Iter16, StorageRef16, T1, S1, T2, S2, T3, S3, T4, S4, T5, S5, T6, S6, T7, S7, T8, S8, T9, S9, T10, S10, T11, S11, T12, S12, T13, S13, T14, S14, T15, S15, T16, S16);
}

impl<'a> IntoIter for &'a [Entity]{
    type Iter = slice::Iter<'a,Entity>;
    fn into_iter(self) -> Self::Iter{
        self.iter()
    }
}

impl<'a> StorageRef<'a, &'a Entity> for &'a [Entity]{
    fn get(&self, guid: usize) -> &'a Entity{
        unsafe{ self.get_unchecked(guid) }
    }

    fn contains(&self, guid: usize) -> bool{
        // TODO: This is slow but will never get called, right?
        self.iter().find(|e| e.guid() == guid).is_some()
    }
}

impl<'a> UnorderedData<'a> for ReadEntities {
    type Iter = slice::Iter<'a,Entity>;
    type Components = Entity;
    type ComponentsRef = &'a Entity;
    type Storage = &'a [Entity];
    fn components_mask(_world: &'a World) -> Bitmask{
        Bitmask::all()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.entities_ref().iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        world.entities_ref()
    }
}

// Thread local Read/Write
pub struct StorageReadLocal<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWriteLocal<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal> StorageRef<'a, <S as Storage<'a,T>>::Get> for StorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> <S as Storage<'a,T>>::Get{
        //unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
        let storage = unsafe{ mem::transmute::<&S, &S>(&self.storage) };
        unsafe{ storage.get(guid) }
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains(guid)
    }
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal> StorageRef<'a, <S as Storage<'a,T>>::GetMut> for StorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> <S as Storage<'a,T>>::GetMut{
        // unsafe{ mem::transmute::<&mut T, &mut T>((*self.storage.get()).get_mut(guid)) }
        let storage = unsafe{ mem::transmute::<&mut S, &mut S>(&mut (*self.storage.get())) };
        unsafe{ storage.get_mut(guid) }
    }

    fn contains(&self, guid: usize) -> bool {
        unsafe{ (*self.storage.get()).contains(guid) }
    }
}

pub trait UnorderedDataLocal<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> Bitmask;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
}

impl<'a, T: 'a + ComponentThreadLocal> UnorderedDataLocal<'a> for Read<'a,T>
    where for<'b> ReadGuardRef<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <ReadGuardRef<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::Get;
    type Storage = StorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_thread_local::<T>().unwrap().into_iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageReadLocal{
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T: 'a + ComponentThreadLocal> UnorderedDataLocal<'a> for Write<'a,T>
    where for<'b> WriteGuardRef<'b, <T as Component>::Storage>: IntoIterMut
{
    type Iter = <WriteGuardRef<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::GetMut;
    type Storage = StorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_thread_local_mut::<T>().unwrap().into_iter_mut()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        StorageWriteLocal{
            storage: UnsafeCell::new(world.storage_thread_local_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
}


impl<'a> UnorderedDataLocal<'a> for ReadEntities {
    type Iter = slice::Iter<'a,Entity>;
    type Components = Entity;
    type ComponentsRef = &'a Entity;
    type Storage = &'a [Entity];
    fn components_mask(_world: &'a World) -> Bitmask{
        Bitmask::all()
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.entities_ref().iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        world.entities_ref()
    }
}


impl<'a, T: 'a + ComponentSync> UnorderedDataLocal<'a> for Not<'a,T> {
    type Iter = iter::Repeat<()>;
    type Components = T;
    type ComponentsRef = ();
    type Storage = ();
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::not(world.components_mask::<T>())
    }

    fn into_iter(_: &'a ::World) -> Self::Iter{
        iter::repeat(())
    }

    fn storage(_: &'a ::World) -> (){
        ()
    }
}


impl<'a, T: 'a + ComponentSync, Not: Component> UnorderedDataLocal<'a> for ReadNot<'a,T,Not> {
    type Iter = ReadNotIter<'a, Self::ComponentsRef, Self::Storage>;
    type Components = T;
    type ComponentsRef = <<T as Component>::Storage as Storage<'a, T>>::Get;
    type Storage = StorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask {
        Bitmask::has_not(world.components_mask::<T>(), world.components_mask::<Not>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter {
        let ids = world.entities_for_mask(<Self as UnorderedDataLocal>::components_mask(world));
        ReadNotIter{
            ptr: ids.index.as_ptr(),
            end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
            _ids: ids,
            storage: <Self as UnorderedDataLocal>::storage(world),
            _marker: marker::PhantomData,
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage {
        StorageReadLocal {
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

//-------------------------------------------------------------------
// Combined iterators
macro_rules! impl_combined_unordered_iter {
    ($iter: ident, $storage_ref: ident, $($t: ident, $s: ident, $u: ident),*) => (

        #[allow(non_snake_case, dead_code)]
        pub struct $iter<'a,$($t, $s:'a,)*>{
            ids: ::IndexGuard<'a>,
            ptr: *const usize,
            end: *const usize,
            $(
                $s: $s,
                $t: marker::PhantomData<$t>,
            )*
        }

        impl<'a, $($t, $s: ::StorageRef<'a, $t> + 'a,)*> Iterator for $iter<'a,$($t, $s,)*>{
            type Item = ($($t),*);
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

                unsafe {
                    if self.ptr == self.end {
                        None
                    } else {
                        let guid = *self.ptr;
                        self.ptr = self.ptr.offset(1);
                        Some(($(self.$s.get(guid)),*))
                    }
                }

                // if self.next == self.ids.index.len(){
                //     None
                // }else{
                //     let guid = unsafe{ *self.ids.index.get_unchecked(self.next) };
                //     self.next += 1;
                //     Some(($(self.$s.get(guid)),*))
                // }
            }
        }

        #[allow(non_snake_case)]
        pub struct $storage_ref<$($s),*>{
            $(
                $s: $s,
            )*
        }

        impl<'a, $($t, $s: ::StorageRef<'a, $t>,)*> ::StorageRef<'a, ($($t),*)> for $storage_ref<$($s),*>{
            fn get(&self, guid: usize) -> ($($t),*){
                ($( self.$s.get(guid) ),*)
            }

            fn contains(&self, guid: usize) -> bool{
               ($( self.$s.contains(guid) ) & *)
            }
        }

        impl<'a, $($u: ::UnorderedData<'a>),* > ::UnorderedData<'a> for ($($u),*)
            where $(
                <$u as ::UnorderedData<'a>>::Storage: 'a,
                $u: 'a,
            )*
        {
            type Iter = $iter<'a, $(
                <$u as ::UnorderedData<'a>>::ComponentsRef, <$u as ::UnorderedData<'a>>::Storage,
            )*>;

            type Components = ($(
                <$u as ::UnorderedData<'a>>::Components
            ),*);

            type ComponentsRef = ($(
                <$u as ::UnorderedData<'a>>::ComponentsRef
            ),*);

            type Storage = $storage_ref<$(
                <$u as ::UnorderedData<'a>>::Storage
            ),*>;

            fn components_mask(world: &'a ::World) -> ::bitmask::Bitmask {
                $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'a ::World) -> Self::Iter{
                let ids = world.entities_for_mask(Self::components_mask(world));
                $iter{
                    // next: 0,
                    ptr: ids.index.as_ptr(),
                    end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                    ids,
                    $(
                        $s: $u::storage(world),
                        $t: marker::PhantomData,
                    )*
                }
            }

            fn storage(world: &'a ::World) -> Self::Storage{
                $storage_ref{
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }
        }

        impl<'a, $($u: ::UnorderedDataLocal<'a>),* > ::UnorderedDataLocal<'a> for ($($u),*)
            where $(
                <$u as ::UnorderedDataLocal<'a>>::Storage: 'a,
                $u: 'a,
            )*
        {
            type Iter = $iter<'a, $(
                <$u as ::UnorderedDataLocal<'a>>::ComponentsRef, <$u as ::UnorderedDataLocal<'a>>::Storage,
            )*>;

            type Components = ($(
                <$u as ::UnorderedDataLocal<'a>>::Components
            ),*);

            type ComponentsRef = ($(
                <$u as ::UnorderedDataLocal<'a>>::ComponentsRef
            ),*);

            type Storage = $storage_ref<$(
                <$u as ::UnorderedDataLocal<'a>>::Storage
            ),*>;

            fn components_mask(world: &'a ::World) -> ::bitmask::Bitmask {
                $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'a ::World) -> Self::Iter{
                let ids = world.entities_for_mask(Self::components_mask(world));
                $iter{
                    // ids: world.entities_for_mask(Self::components_mask(world)),
                    // next: 0,
                    ptr: ids.index.as_ptr(),
                    end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                    ids,
                    $(
                        $s: $u::storage(world),
                        $t: marker::PhantomData,
                    )*
                }
            }

            fn storage(world: &'a ::World) -> Self::Storage{
                $storage_ref{
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }
        }
    )
}

mod combined_unordered{
    use std::marker;
    impl_combined_unordered_iter!(Iter2, StorageRef2, T1, S1, U1, T2, S2, U2);
    impl_combined_unordered_iter!(Iter3, StorageRef3, T1, S1, U1, T2, S2, U2, T3, S3, U3);
    impl_combined_unordered_iter!(Iter4, StorageRef4, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4);
    impl_combined_unordered_iter!(Iter5, StorageRef5, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5);
    impl_combined_unordered_iter!(Iter6, StorageRef6, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6);
    impl_combined_unordered_iter!(Iter7, StorageRef7, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7);
    impl_combined_unordered_iter!(Iter8, StorageRef8, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8);
    impl_combined_unordered_iter!(Iter9, StorageRef9, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9);
    impl_combined_unordered_iter!(Iter10, StorageRef10, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10);
    impl_combined_unordered_iter!(Iter11, StorageRef11, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11);
    impl_combined_unordered_iter!(Iter12, StorageRef12, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12);
    impl_combined_unordered_iter!(Iter13, StorageRef13, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13);
    impl_combined_unordered_iter!(Iter14, StorageRef14, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14);
    impl_combined_unordered_iter!(Iter15, StorageRef15, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14, T15, S15, U15);
    impl_combined_unordered_iter!(Iter16, StorageRef16, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14, T15, S15, U15, T16, S16, U16);
}




use idtree;
use forest;


pub trait IntoOrderedIter{
    type OrderedIter: Iterator;
    fn into_ordered_iter(self) -> Self::OrderedIter;
}

pub trait IntoOrderedIterMut{
    type OrderedIterMut: Iterator;
    fn into_ordered_iter_mut(self) -> Self::OrderedIterMut;
}

pub trait IntoHierarchicalIter<'a, T>{
    fn into_hierarchical_iter(self) -> forest::ForestHierarchicalIter<'a, T>;
}

pub trait IntoHierarchicalIterMut<'a, T>{
    fn into_hierarchical_iter_mut(self) -> forest::ForestHierarchicalIterMut<'a, T>;
}

impl<'a,T:'a,I> IntoHierarchicalIter<'a,T> for I
    where I: IntoOrderedIter<OrderedIter = forest::ForestHierarchicalIter<'a,T>>
{
    fn into_hierarchical_iter(self) -> forest::ForestHierarchicalIter<'a, T>{
        self.into_ordered_iter()
    }
}

impl<'a,T:'a,I> IntoHierarchicalIterMut<'a,T> for I
    where I: IntoOrderedIterMut<OrderedIterMut = forest::ForestHierarchicalIterMut<'a,T>>
{
    fn into_hierarchical_iter_mut(self) -> forest::ForestHierarchicalIterMut<'a, T>{
        self.into_ordered_iter_mut()
    }
}

pub trait HierarchicalStorage<'a,T>: Storage<'a,T>{
    unsafe fn insert_child(&mut self, parent_guid: usize, guid: usize, value: T);
    unsafe fn get_node(&self, guid: usize) -> idtree::NodeRef<T>;
    unsafe fn get_node_mut(&mut self, guid: usize) -> idtree::NodeRefMut<T>;
    fn ordered_ids(&self) -> &[usize];
}

pub struct ReadHierarchical<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteHierarchical<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}


// Sync Hierarchical Read/Write
pub struct HierarchicalStorageRead<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWrite<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeRef<'a, T>> for HierarchicalStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) }
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains(guid)
    }
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeRefMut<'a, T>> for HierarchicalStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRefMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }

    fn contains(&self, guid: usize) -> bool {
        unsafe{ (*self.storage.get()).contains(guid) }
    }
}


pub trait OrderedData<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> Bitmask;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard;
}


impl<'a, T: 'a + ComponentSync> OrderedData<'a> for ReadHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoOrderedIter
{
    type Iter = <RwLockReadGuard<'a, <T as Component>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeRef<'a, T>;
    type Storage = HierarchicalStorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage::<T>().unwrap().into_ordered_iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        HierarchicalStorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}


impl<'a, T: 'a + ComponentSync> OrderedData<'a> for WriteHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoOrderedIterMut
{
    type Iter = <RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeRefMut<'a, T>;
    type Storage = HierarchicalStorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_mut::<T>().unwrap().into_ordered_iter_mut()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        HierarchicalStorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}

// Local Hierarchical Read/Write
pub struct HierarchicalStorageReadLocal<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWriteLocal<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeRef<'a, T>> for HierarchicalStorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) }
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains(guid)
    }
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeRefMut<'a, T>> for HierarchicalStorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRefMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }

    fn contains(&self, guid: usize) -> bool {
        unsafe{ (*self.storage.get()).contains(guid) }
    }
}


pub trait OrderedDataLocal<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> Bitmask;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard;
}


impl<'a, T: 'a + ComponentSync> OrderedDataLocal<'a> for ReadHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> ReadGuardRef<'b, <T as Component>::Storage>: IntoOrderedIter
{
    type Iter = <ReadGuardRef<'a, <T as Component>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeRef<'a, T>;
    type Storage = HierarchicalStorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_thread_local::<T>().unwrap().into_ordered_iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        HierarchicalStorageReadLocal{
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.thread_local_ordered_entities_for::<T>(mask)
    }
}


impl<'a, T: 'a + ComponentSync> OrderedDataLocal<'a> for WriteHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> WriteGuardRef<'b, <T as Component>::Storage>: IntoOrderedIterMut
{
    type Iter = <WriteGuardRef<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeRefMut<'a, T>;
    type Storage = HierarchicalStorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.storage_thread_local_mut::<T>().unwrap().into_ordered_iter_mut()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        HierarchicalStorageWriteLocal{
            storage: UnsafeCell::new(world.storage_thread_local_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.thread_local_ordered_entities_for::<T>(mask)
    }
}


//-------------------------------------------------------------------
// Combined iterators
macro_rules! impl_combined_ordered_iter {
    ($iter: ident, $storage_ref: ident, $to:ident, $so:ident, $uo:ident, $($t: ident, $s: ident, $u: ident),*) => (

        #[allow(non_snake_case, dead_code)]
        pub struct $iter<'a,$to,$so,$($t, $s:'a,)*>{
            ids: ::IndexGuard<'a>,
            ptr: *const usize,
            end: *const usize,
            $so: $so,
            $to: marker::PhantomData<$to>,
            $(
                $s: $s,
                $t: marker::PhantomData<$t>,
            )*
        }

        impl<'a, $to, $so: ::StorageRef<'a,$to> + 'a, $($t, $s: ::StorageRef<'a, $t> + 'a,)*> Iterator for $iter<'a, $to, $so, $($t, $s,)*>{
            type Item = ($to,$($t),*);
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

                unsafe {
                    if self.ptr == self.end {
                        None
                    } else {
                        let guid = *self.ptr;
                        self.ptr = self.ptr.offset(1);
                        Some((self.$so.get(guid), $(self.$s.get(guid)),*))
                    }
                }

                // if self.next == self.ids.index.len(){
                //     None
                // }else{
                //     let guid = unsafe{ *self.ids.index.get_unchecked(self.next) };
                //     self.next += 1;
                //     Some(($(self.$s.get(guid)),*))
                // }
            }
        }

        #[allow(non_snake_case)]
        pub struct $storage_ref<$so, $($s),*>{
            $so: $so,
            $(
                $s: $s,
            )*
        }

        impl<'a, $to, $so: ::StorageRef<'a, $to>, $($t, $s: ::StorageRef<'a, $t>,)*> ::StorageRef<'a, ($to, $($t),*)> for $storage_ref<$so, $($s),*>{
            fn get(&self, guid: usize) -> ($to, $($t),*){
                (self.$so.get(guid), $( self.$s.get(guid) ),*)
            }

            fn contains(&self, guid: usize) -> bool{
                self.$so.contains(guid) & $( self.$s.contains(guid) )&*
            }
        }

        impl<'a, $uo: ::OrderedData<'a>, $($u: ::UnorderedData<'a>),* > ::OrderedData<'a> for ($uo, $($u),*)
            where
            <$uo as ::OrderedData<'a>>::Storage: 'a,
            $uo: 'a,
            $(
                <$u as ::UnorderedData<'a>>::Storage: 'a,
                $u: 'a,
            )*
        {
            type Iter = $iter<'a,
                <$uo as ::OrderedData<'a>>::ComponentsRef, <$uo as ::OrderedData<'a>>::Storage,
                $(
                    <$u as ::UnorderedData<'a>>::ComponentsRef, <$u as ::UnorderedData<'a>>::Storage,
                )*>;

            type Components =
                (<$uo as ::OrderedData<'a>>::Components,
                $(
                    <$u as ::UnorderedData<'a>>::Components
                ),*);

            type ComponentsRef =
                (<$uo as ::OrderedData<'a>>::ComponentsRef,
                $(
                    <$u as ::UnorderedData<'a>>::ComponentsRef
                ),*);

            type Storage = $storage_ref<
                <$uo as ::OrderedData<'a>>::Storage,
                $(
                    <$u as ::UnorderedData<'a>>::Storage
                ),*>;

            fn components_mask(world: &'a ::World) -> ::bitmask::Bitmask {
                $uo::components_mask(world) | $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'a ::World) -> Self::Iter{
                let ids = $uo::ordered_ids(world, Self::components_mask(world));
                $iter{
                    // next: 0,
                    ptr: ids.index.as_ptr(),
                    end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                    ids,
                    $so: $uo::storage(world),
                    $to: marker::PhantomData,
                    $(
                        $s: $u::storage(world),
                        $t: marker::PhantomData,
                    )*
                }
            }

            fn storage(world: &'a ::World) -> Self::Storage{
                $storage_ref{
                    $so: $uo::storage(world),
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }

            fn ordered_ids(world: &'a ::World, mask: ::bitmask::Bitmask) -> IndexGuard{
                $uo::ordered_ids(world, mask)
            }
        }

        impl<'a, $uo: ::OrderedDataLocal<'a>, $($u: ::UnorderedDataLocal<'a>),* > ::OrderedDataLocal<'a> for ($uo, $($u),*)
            where
                <$uo as ::OrderedDataLocal<'a>>::Storage: 'a,
                $uo: 'a,
                $(
                    <$u as ::UnorderedDataLocal<'a>>::Storage: 'a,
                    $u: 'a,
                )*
        {
            type Iter = $iter<'a,
                <$uo as ::OrderedDataLocal<'a>>::ComponentsRef, <$uo as ::OrderedDataLocal<'a>>::Storage,
                $(
                    <$u as ::UnorderedDataLocal<'a>>::ComponentsRef, <$u as ::UnorderedDataLocal<'a>>::Storage,
                )*>;

            type Components = (
                <$uo as ::OrderedDataLocal<'a>>::Components,
                $(
                    <$u as ::UnorderedDataLocal<'a>>::Components
                ),*);

            type ComponentsRef = (
                <$uo as ::OrderedDataLocal<'a>>::ComponentsRef,
                $(
                    <$u as ::UnorderedDataLocal<'a>>::ComponentsRef
                ),*);

            type Storage = $storage_ref<
                <$uo as ::OrderedDataLocal<'a>>::Storage,
                $(
                    <$u as ::UnorderedDataLocal<'a>>::Storage
                ),*>;

            fn components_mask(world: &'a ::World) -> ::bitmask::Bitmask {
                $uo::components_mask(world) | $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'a ::World) -> Self::Iter{
                let ids = world.entities_for_mask(Self::components_mask(world));
                $iter{
                    // ids: world.entities_for_mask(Self::components_mask(world)),
                    // next: 0,
                    ptr: ids.index.as_ptr(),
                    end: unsafe{ ids.index.as_ptr().offset(ids.index.len() as isize) },
                    ids,
                    $so: $uo::storage(world),
                    $to: marker::PhantomData,
                    $(
                        $s: $u::storage(world),
                        $t: marker::PhantomData,
                    )*
                }
            }

            fn storage(world: &'a ::World) -> Self::Storage{
                $storage_ref{
                    $so: $uo::storage(world),
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }

            fn ordered_ids(world: &'a ::World, mask: ::bitmask::Bitmask) -> IndexGuard{
                $uo::ordered_ids(world, mask)
            }
        }
    )
}

mod combined_ordered{
    use std::marker;
    use ::IndexGuard;
    impl_combined_ordered_iter!(Iter2, StorageRef2, T1, S1, U1, T2, S2, U2);
    impl_combined_ordered_iter!(Iter3, StorageRef3, T1, S1, U1, T2, S2, U2, T3, S3, U3);
    impl_combined_ordered_iter!(Iter4, StorageRef4, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4);
    impl_combined_ordered_iter!(Iter5, StorageRef5, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5);
    impl_combined_ordered_iter!(Iter6, StorageRef6, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6);
    impl_combined_ordered_iter!(Iter7, StorageRef7, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7);
    impl_combined_ordered_iter!(Iter8, StorageRef8, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8);
    impl_combined_ordered_iter!(Iter9, StorageRef9, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9);
    impl_combined_ordered_iter!(Iter10, StorageRef10, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10);
    impl_combined_ordered_iter!(Iter11, StorageRef11, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11);
    impl_combined_ordered_iter!(Iter12, StorageRef12, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12);
    impl_combined_ordered_iter!(Iter13, StorageRef13, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13);
    impl_combined_ordered_iter!(Iter14, StorageRef14, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14);
    impl_combined_ordered_iter!(Iter15, StorageRef15, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14, T15, S15, U15);
    impl_combined_ordered_iter!(Iter16, StorageRef16, T1, S1, U1, T2, S2, U2, T3, S3, U3, T4, S4, U4, T5, S5, U5, T6, S6, U6, T7, S7, U7, T8, S8, U8, T9, S9, U9, T10, S10, U10, T11, S11, U11, T12, S12, U12, T13, S13, U13, T14, S14, U14, T15, S15, U15, T16, S16, U16);
}


use forest::ForestHierarchicalIterMut;
use forest::ForestHierarchicalIter;

pub struct ReadAndParent<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteAndParent<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}


pub struct ParentStorageRead<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, (&'a T, Option<&'a T>)> for ParentStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a T, Option<&'a T>){
        let node = unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&T, Option<&T>), (&T, Option<&T>)>((&node, parent)) }
    }

    fn contains(&self, guid: usize) -> bool {
        self.storage.contains(guid)
    }
}

pub struct ReadAndParentIter<'a, T: Component>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoHierarchicalIter<'b,T>
{
    it: ForestHierarchicalIter<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for ReadAndParentIter<'a, T>
    where T: Component,
          <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoHierarchicalIter<'b,T>,
{
    type Item = (&'a T, Option<&'a T>);
    fn next(&mut self) -> Option<(&'a T, Option<&'a T>)>{
        self.it.next().map(|n| {
            let n_ref = unsafe{ mem::transmute::<&T, &T>(&n) };
            let p = n.parent().map(|p|  unsafe{ mem::transmute::<&T, &T>(&p) });
            (n_ref, p)
        })
    }
}


impl<'a, T: 'a + ComponentSync> OrderedData<'a> for ReadAndParent<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoHierarchicalIter<'b,T>
{
    type Iter = ReadAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a T, Option<&'a T>);
    type Storage = ParentStorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        ReadAndParentIter{
            it: world.storage::<T>().unwrap().into_hierarchical_iter()
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        ParentStorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}



pub struct ParentStorageWrite<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync> StorageRef<'a, (&'a mut T, Option<&'a T>)> for ParentStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a mut T, Option<&'a T>){
        let mut node = unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&mut T, Option<&T>), (&mut T, Option<&T>)>((&mut node, parent)) }
    }

    fn contains(&self, guid: usize) -> bool {
        unsafe{ (*self.storage.get()).contains(guid) }
    }
}

pub struct WriteAndParentIter<'a, T: Component>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoHierarchicalIterMut<'b,T>
{
    it: ForestHierarchicalIterMut<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for WriteAndParentIter<'a, T>
    where T: Component,
          <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoHierarchicalIterMut<'b,T>,
{
    type Item = (&'a mut T, Option<&'a T>);
    fn next(&mut self) -> Option<(&'a mut T, Option<&'a T>)>{
        self.it.next().map(|mut n| {
            let n_ref = unsafe{ mem::transmute::<&mut T, &mut T>(&mut n) };
            let p = unsafe{ mem::transmute::<Option<&T>, Option<&T>>(n.parent().map(|p| p)) };
            (n_ref, p)
        })
    }
}

impl<'a, T: 'a + ComponentSync> OrderedData<'a> for WriteAndParent<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoHierarchicalIterMut<'b,T>
{
    type Iter = WriteAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a mut T, Option<&'a T>);
    type Storage = ParentStorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> Bitmask{
        Bitmask::has(world.components_mask::<T>())
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        WriteAndParentIter{
            it: world.storage_mut::<T>().unwrap().into_hierarchical_iter_mut()
        }
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        ParentStorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'a ::World, mask: Bitmask) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}


// OneToN
// use component::{OneToNComponent, OneToNComponentSync, OneToNComponentThreadLocal};

pub trait OneToNStorage<'a,T>: Storage<'a,T>{
    fn insert_slice(&mut self, guid: usize, t: &[T]) where T: Clone;
    unsafe fn get_slice(&self, guid: usize) -> &[T];
    unsafe fn get_slice_mut(&mut self, guid: usize) -> &mut [T];
}


// OneToNHierarchical
pub trait HierarchicalOneToNStorage<'a,T>: Storage<'a,T>{
    unsafe fn insert_root(&mut self, guid: usize, t: T) -> idtree::NodeRefMut<T>;
    unsafe fn insert_child(&mut self, parent: idtree::NodeId, t: T) -> idtree::NodeRefMut<T>;
}

// pub struct ReadOneToN<'a, T: 'a + OneToNComponent>{
//     marker: marker::PhantomData<&'a T>,
// }
//
//
// pub struct WriteOneToN<'a, T: 'a + OneToNComponent>{
//     marker: marker::PhantomData<&'a T>,
// }
//
//
// impl<'a, T: 'a + OneToNComponentSync> UnorderedData<'a> for ReadOneToN<'a,T>
//     where for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoIter
// {
//     type Iter = <RwLockReadGuard<'a, <T as Component>::Storage> as IntoIter>::Iter;
//     type Components = T;
//     type ComponentsRef = &'a [T];
//     type Storage = StorageRead<'a, <T as Component>::Storage, Self::Components>;
//     fn components_mask(world: &'a World) -> usize{
//         world.components_mask::<T>()
//     }
//
//     fn into_iter(world: &'a ::World) -> Self::Iter{
//         world.storage::<T>().unwrap().into_iter()
//     }
//
//     fn storage(world: &'a ::World) -> Self::Storage{
//         StorageRead{
//             storage: world.storage::<T>().unwrap(),
//             _marker: marker::PhantomData,
//         }
//     }
// }
//
//
// impl<'a, T: 'a + OneToNComponentSync> UnorderedData<'a> for WriteOneToN<'a,T>
//     where for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoIterMut
// {
//     type Iter = <RwLockWriteGuard<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
//     type Components = T;
//     type ComponentsRef = &'a mut [T];
//     type Storage = StorageWrite<'a, <T as Component>::Storage, Self::Components>;
//     fn components_mask(world: &'a World) -> usize{
//         world.components_mask::<T>()
//     }
//
//     fn into_iter(world: &'a ::World) -> Self::Iter{
//         world.storage_mut::<T>().unwrap().into_iter_mut()
//     }
//
//     fn storage(world: &'a ::World) -> Self::Storage{
//         StorageWrite{
//             storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
//             _marker: marker::PhantomData,
//         }
//     }
// }
//
//
// impl<'a, T: 'a + OneToNComponentThreadLocal> UnorderedDataLocal<'a> for ReadOneToN<'a,T>
//     where for<'b> ReadGuardRef<'b, <T as Component>::Storage>: IntoIter
// {
//     type Iter = <ReadGuardRef<'a, <T as Component>::Storage> as IntoIter>::Iter;
//     type Components = T;
//     type ComponentsRef = &'a [T];
//     type Storage = StorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
//     fn components_mask(world: &'a World) -> usize{
//         world.components_mask::<T>()
//     }
//
//     fn into_iter(world: &'a ::World) -> Self::Iter{
//         world.storage_thread_local::<T>().unwrap().into_iter()
//     }
//
//     fn storage(world: &'a ::World) -> Self::Storage{
//         StorageReadLocal{
//             storage: world.storage_thread_local::<T>().unwrap(),
//             _marker: marker::PhantomData,
//         }
//     }
// }
//
//
// impl<'a, T: 'a + OneToNComponentThreadLocal> UnorderedDataLocal<'a> for WriteOneToN<'a,T>
//     where for<'b> WriteGuardRef<'b, <T as Component>::Storage>: IntoIterMut
// {
//     type Iter = <WriteGuardRef<'a, <T as Component>::Storage> as IntoIterMut>::IterMut;
//     type Components = T;
//     type ComponentsRef = &'a mut [T];
//     type Storage = StorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
//     fn components_mask(world: &'a World) -> usize{
//         world.components_mask::<T>()
//     }
//
//     fn into_iter(world: &'a ::World) -> Self::Iter{
//         world.storage_thread_local_mut::<T>().unwrap().into_iter_mut()
//     }
//
//     fn storage(world: &'a ::World) -> Self::Storage{
//         StorageWriteLocal{
//             storage: UnsafeCell::new(world.storage_thread_local_mut::<T>().unwrap()),
//             _marker: marker::PhantomData,
//         }
//     }
// }
