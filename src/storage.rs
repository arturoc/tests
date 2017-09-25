use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::cell::{RefCell, UnsafeCell};
use std::mem;
use std::slice;

use sync::{ReadGuardRef, WriteGuardRef};
use ::Component;
use ::ComponentSync;
use ::ComponentThreadLocal;
use ::World;
use ::IndexGuard;
use ::Entity;

pub unsafe trait AnyStorage{}


impl AnyStorage{
    #[inline]
    pub unsafe fn downcast_ref<'a, T: AnyStorage + 'a>(&self) -> &'a T {
        &*(self as *const AnyStorage as *const T)
    }
}

unsafe impl<S: AnyStorage> AnyStorage for RwLock<S>{}
unsafe impl<S: AnyStorage> AnyStorage for RefCell<S>{}

pub trait Storage<'a,T>: AnyStorage{
    type Get: ?Sized;
    fn new() -> Self;
    fn with_capacity(usize) -> Self;
    fn insert(&mut self, guid: usize, t: T);
    fn remove(&mut self, guid: usize);
    unsafe fn get(&'a self, guid: usize) -> &'a Self::Get;
    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut Self::Get;
}

pub trait IntoIter{
    type Iter: Iterator;
    fn into_iter(self) -> Self::Iter;
}

pub trait IntoIterMut{
    type IterMut: Iterator;
    fn into_iter_mut(self) -> Self::IterMut;
}

pub struct Read<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct Write<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct ReadEntities;

// Sync Read/Write
pub struct StorageRead<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWrite<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

pub trait StorageRef<'a, T>{
    fn get(&self, guid: usize) -> T;
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, &'a <S as Storage<'a,T>>::Get> for StorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> &'a <S as Storage<'a,T>>::Get{
        // unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }

        let storage = unsafe{ mem::transmute::<&S, &S>(&self.storage) };
        unsafe{ storage.get(guid) }
    }
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, &'a mut <S as Storage<'a,T>>::Get> for StorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> &'a mut <S as Storage<'a,T>>::Get{
        // unsafe{ mem::transmute::<&mut T, &mut T>( (*self.storage.get()).get_mut(guid) ) }


        let storage = unsafe{ mem::transmute::<&mut S, &mut S>(&mut (*self.storage.get())) };
        unsafe{ storage.get_mut(guid) }
    }
}

pub trait UnorderedData<'a, 'b> where 'a: 'b{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'b World<'a>) -> usize;
    fn into_iter(world: &'b ::World<'a>) -> Self::Iter;
    fn storage(world: &'b ::World<'a>) -> Self::Storage;
}

impl<'a, 'b, T: 'a + ComponentSync<'a>> UnorderedData<'a, 'b> for Read<'a,T>
    where for<'c> RwLockReadGuard<'c, <T as Component<'a>>::Storage>: IntoIter,
    'a: 'b,
{
    type Iter = <RwLockReadGuard<'a, <T as Component<'a>>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a <<T as Component<'a>>::Storage as Storage<'a,T>>::Get;
    type Storage = StorageRead<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage::<T>().unwrap().into_iter()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        StorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, 'b, T: 'a + ComponentSync<'a>> UnorderedData<'a, 'b> for Write<'a,T>
    where for<'c> RwLockWriteGuard<'c, <T as Component<'a>>::Storage>: IntoIterMut,
    'a: 'b,
{
    type Iter = <RwLockWriteGuard<'a, <T as Component<'a>>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = &'a mut <<T as Component<'a>>::Storage as Storage<'a,T>>::Get;
    type Storage = StorageWrite<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_mut::<T>().unwrap().into_iter_mut()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        StorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
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
}

impl<'a, 'b> UnorderedData<'a, 'b> for ReadEntities
    where 'a: 'b,
{
    type Iter = slice::Iter<'b,Entity>;
    type Components = Entity;
    type ComponentsRef = &'b Entity;
    type Storage = &'b [Entity];
    fn components_mask(_world: &'b World<'a>) -> usize{
        0
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.entities_ref().iter()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        world.entities_ref()
    }
}

// Thread local Read/Write
pub struct StorageReadLocal<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWriteLocal<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>> StorageRef<'a, &'a <S as Storage<'a,T>>::Get> for StorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a <S as Storage<'a,T>>::Get{
        //unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
        let storage = unsafe{ mem::transmute::<&S, &S>(&self.storage) };
        unsafe{ storage.get(guid) }
    }
}

impl<'a, S: Storage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>> StorageRef<'a, &'a mut <S as Storage<'a,T>>::Get> for StorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a mut <S as Storage<'a,T>>::Get{
        // unsafe{ mem::transmute::<&mut T, &mut T>((*self.storage.get()).get_mut(guid)) }
        let storage = unsafe{ mem::transmute::<&mut S, &mut S>(&mut (*self.storage.get())) };
        unsafe{ storage.get_mut(guid) }
    }
}

pub trait UnorderedDataLocal<'a, 'b> where 'a: 'b{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'b World<'a>) -> usize;
    fn into_iter(world: &'b ::World<'a>) -> Self::Iter;
    fn storage(world: &'b ::World<'a>) -> Self::Storage;
}

impl<'a, 'b, T: 'a + ComponentThreadLocal<'a>> UnorderedDataLocal<'a, 'b> for Read<'a,T>
    where for<'c> ReadGuardRef<'c, <T as Component<'a>>::Storage>: IntoIter,
    'a: 'b,
{
    type Iter = <ReadGuardRef<'a, <T as Component<'a>>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a <<T as Component<'a>>::Storage as Storage<'a,T>>::Get;
    type Storage = StorageReadLocal<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_thread_local::<T>().unwrap().into_iter()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        StorageReadLocal{
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, 'b, T: 'a + ComponentThreadLocal<'a>> UnorderedDataLocal<'a, 'b> for Write<'a,T>
    where for<'c> WriteGuardRef<'c, <T as Component<'a>>::Storage>: IntoIterMut,
    'a: 'b,
{
    type Iter = <WriteGuardRef<'a, <T as Component<'a>>::Storage> as IntoIterMut>::IterMut;
    type Components = T;
    type ComponentsRef = &'a mut <<T as Component<'a>>::Storage as Storage<'a,T>>::Get;
    type Storage = StorageWriteLocal<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_thread_local_mut::<T>().unwrap().into_iter_mut()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        StorageWriteLocal{
            storage: UnsafeCell::new(world.storage_thread_local_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }
}



//-------------------------------------------------------------------
// Combined iterators
macro_rules! impl_combined_unordered_iter {
    ($iter: ident, $storage_ref: ident, $($t: ident, $s: ident, $u: ident),*) => (

        #[allow(non_snake_case, dead_code)]
        pub struct $iter<'a,'b,$($t, $s:'b,)*>{
            ids: ::IndexGuard<'b>,
            ptr: *const usize,
            end: *const usize,
            $(
                $s: $s,
                $t: marker::PhantomData<$t>,
            )*
            marker: marker::PhantomData<&'a usize>
        }

        impl<'a,'b, $($t, $s: ::StorageRef<'a, $t> + 'a,)*> Iterator for $iter<'a,'b,$($t, $s,)*>{
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
        }

        impl<'a, 'b, $($u: ::UnorderedData<'a, 'b>),* > ::UnorderedData<'a, 'b> for ($($u),*)
            where $(
                <$u as ::UnorderedData<'a, 'b>>::Storage: 'b,
                $u: 'b,
            )*
            'a: 'b,
        {
            type Iter = $iter<'a, 'b, $(
                <$u as ::UnorderedData<'a, 'b>>::ComponentsRef, <$u as ::UnorderedData<'a, 'b>>::Storage,
            )*>;

            type Components = ($(
                <$u as ::UnorderedData<'a, 'b>>::Components
            ),*);

            type ComponentsRef = ($(
                <$u as ::UnorderedData<'a, 'b>>::ComponentsRef
            ),*);

            type Storage = $storage_ref<$(
                <$u as ::UnorderedData<'a, 'b>>::Storage
            ),*>;

            fn components_mask(world: &'b ::World<'a>) -> usize {
                $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
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
                    marker: marker::PhantomData
                }
            }

            fn storage(world: &'b ::World<'a>) -> Self::Storage{
                $storage_ref{
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }
        }

        impl<'a, 'b, $($u: ::UnorderedDataLocal<'a, 'b>),* > ::UnorderedDataLocal<'a, 'b> for ($($u),*)
            where $(
                <$u as ::UnorderedDataLocal<'a, 'b>>::Storage: 'b,
                $u: 'b,
            )*
            'a: 'b
        {
            type Iter = $iter<'a,'b, $(
                <$u as ::UnorderedDataLocal<'a, 'b>>::ComponentsRef, <$u as ::UnorderedDataLocal<'a, 'b>>::Storage,
            )*>;

            type Components = ($(
                <$u as ::UnorderedDataLocal<'a, 'b>>::Components
            ),*);

            type ComponentsRef = ($(
                <$u as ::UnorderedDataLocal<'a, 'b>>::ComponentsRef
            ),*);

            type Storage = $storage_ref<$(
                <$u as ::UnorderedDataLocal<'a, 'b>>::Storage
            ),*>;

            fn components_mask(world: &'b ::World<'a>) -> usize {
                $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
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
                    marker: marker::PhantomData
                }
            }

            fn storage(world: &'b ::World<'a>) -> Self::Storage{
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

pub struct ReadHierarchical<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteHierarchical<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}


// Sync Hierarchical Read/Write
pub struct HierarchicalStorageRead<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWrite<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, idtree::NodeRef<'a, T>> for HierarchicalStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) }
    }
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, idtree::NodeRefMut<'a, T>> for HierarchicalStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRefMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }
}


pub trait OrderedData<'a, 'b> where 'a: 'b{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'b World<'a>) -> usize;
    fn into_iter(world: &'b ::World<'a>) -> Self::Iter;
    fn storage(world: &'b ::World<'a>) -> Self::Storage;
    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>;
}


impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedData<'a, 'b> for ReadHierarchical<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> RwLockReadGuard<'c, <T as Component<'a>>::Storage>: IntoOrderedIter,
          'a: 'b,
{
    type Iter = <RwLockReadGuard<'a, <T as Component<'a>>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeRef<'a, T>;
    type Storage = HierarchicalStorageRead<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage::<T>().unwrap().into_ordered_iter()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        HierarchicalStorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
        world.ordered_entities_for::<T>(mask)
    }
}


impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedData<'a, 'b> for WriteHierarchical<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> RwLockWriteGuard<'c, <T as Component<'a>>::Storage>: IntoOrderedIterMut,
          'a: 'b,
{
    type Iter = <RwLockWriteGuard<'a, <T as Component<'a>>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeRefMut<'a, T>;
    type Storage = HierarchicalStorageWrite<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_mut::<T>().unwrap().into_ordered_iter_mut()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        HierarchicalStorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
        world.ordered_entities_for::<T>(mask)
    }
}

// Local Hierarchical Read/Write
pub struct HierarchicalStorageReadLocal<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWriteLocal<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentThreadLocal<'a>>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, idtree::NodeRef<'a, T>> for HierarchicalStorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) }
    }
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, idtree::NodeRefMut<'a, T>> for HierarchicalStorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeRefMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }
}


pub trait OrderedDataLocal<'a,'b> where 'a: 'b{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'b World<'a>) -> usize;
    fn into_iter(world: &'b ::World<'a>) -> Self::Iter;
    fn storage(world: &'b ::World<'a>) -> Self::Storage;
    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>;
}


impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedDataLocal<'a, 'b> for ReadHierarchical<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> ReadGuardRef<'c, <T as Component<'a>>::Storage>: IntoOrderedIter,
          'a: 'b,
{
    type Iter = <ReadGuardRef<'a, <T as Component<'a>>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeRef<'a, T>;
    type Storage = HierarchicalStorageReadLocal<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_thread_local::<T>().unwrap().into_ordered_iter()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        HierarchicalStorageReadLocal{
            storage: world.storage_thread_local::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
        world.thread_local_ordered_entities_for::<T>(mask)
    }
}


impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedDataLocal<'a,'b> for WriteHierarchical<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> WriteGuardRef<'c, <T as Component<'a>>::Storage>: IntoOrderedIterMut,
          'a: 'b,
{
    type Iter = <WriteGuardRef<'a, <T as Component<'a>>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeRefMut<'a, T>;
    type Storage = HierarchicalStorageWriteLocal<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        world.storage_thread_local_mut::<T>().unwrap().into_ordered_iter_mut()
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        HierarchicalStorageWriteLocal{
            storage: UnsafeCell::new(world.storage_thread_local_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
        world.thread_local_ordered_entities_for::<T>(mask)
    }
}


//-------------------------------------------------------------------
// Combined iterators
macro_rules! impl_combined_ordered_iter {
    ($iter: ident, $storage_ref: ident, $to:ident, $so:ident, $uo:ident, $($t: ident, $s: ident, $u: ident),*) => (

        #[allow(non_snake_case, dead_code)]
        pub struct $iter<'a,'b, $to,$so,$($t, $s:'b,)*>{
            ids: ::IndexGuard<'b>,
            ptr: *const usize,
            end: *const usize,
            $so: $so,
            $to: marker::PhantomData<$to>,
            $(
                $s: $s,
                $t: marker::PhantomData<$t>,
            )*
            marker: marker::PhantomData<&'a usize>
        }

        impl<'a, 'b, $to, $so: ::StorageRef<'a,$to> + 'a, $($t, $s: ::StorageRef<'a, $t> + 'a,)*> Iterator for $iter<'a, 'b, $to, $so, $($t, $s,)*>{
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
        }

        impl<'a, 'b, $uo: ::OrderedData<'a, 'b>, $($u: ::UnorderedData<'a, 'b>),* > ::OrderedData<'a, 'b> for ($uo, $($u),*)
            where
            <$uo as ::OrderedData<'a, 'b>>::Storage: 'b,
            $uo: 'b,
            $(
                <$u as ::UnorderedData<'a, 'b>>::Storage: 'b,
                $u: 'b,
            )*
            'a: 'b
        {
            type Iter = $iter<'a, 'b,
                <$uo as ::OrderedData<'a, 'b>>::ComponentsRef, <$uo as ::OrderedData<'a, 'b>>::Storage,
                $(
                    <$u as ::UnorderedData<'a, 'b>>::ComponentsRef, <$u as ::UnorderedData<'a, 'b>>::Storage,
                )*>;

            type Components =
                (<$uo as ::OrderedData<'a, 'b>>::Components,
                $(
                    <$u as ::UnorderedData<'a, 'b>>::Components
                ),*);

            type ComponentsRef =
                (<$uo as ::OrderedData<'a, 'b>>::ComponentsRef,
                $(
                    <$u as ::UnorderedData<'a, 'b>>::ComponentsRef
                ),*);

            type Storage = $storage_ref<
                <$uo as ::OrderedData<'a, 'b>>::Storage,
                $(
                    <$u as ::UnorderedData<'a, 'b>>::Storage
                ),*>;

            fn components_mask(world: &'b ::World<'a>) -> usize {
                $uo::components_mask(world) | $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
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
                    marker: marker::PhantomData,
                }
            }

            fn storage(world: &'b ::World<'a>) -> Self::Storage{
                $storage_ref{
                    $so: $uo::storage(world),
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }

            fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
                $uo::ordered_ids(world, mask)
            }
        }

        impl<'a, 'b, $uo: ::OrderedDataLocal<'a, 'b>, $($u: ::UnorderedDataLocal<'a, 'b>),* > ::OrderedDataLocal<'a, 'b> for ($uo, $($u),*)
            where
                <$uo as ::OrderedDataLocal<'a, 'b>>::Storage: 'b,
                $uo: 'b,
                $(
                    <$u as ::UnorderedDataLocal<'a, 'b>>::Storage: 'b,
                    $u: 'b,
                )*
                'a: 'b,
        {
            type Iter = $iter<'a,'b,
                <$uo as ::OrderedDataLocal<'a, 'b>>::ComponentsRef, <$uo as ::OrderedDataLocal<'a, 'b>>::Storage,
                $(
                    <$u as ::UnorderedDataLocal<'a, 'b>>::ComponentsRef, <$u as ::UnorderedDataLocal<'a, 'b>>::Storage,
                )*>;

            type Components = (
                <$uo as ::OrderedDataLocal<'a, 'b>>::Components,
                $(
                    <$u as ::UnorderedDataLocal<'a, 'b>>::Components
                ),*);

            type ComponentsRef = (
                <$uo as ::OrderedDataLocal<'a, 'b>>::ComponentsRef,
                $(
                    <$u as ::UnorderedDataLocal<'a, 'b>>::ComponentsRef
                ),*);

            type Storage = $storage_ref<
                <$uo as ::OrderedDataLocal<'a, 'b>>::Storage,
                $(
                    <$u as ::UnorderedDataLocal<'a, 'b>>::Storage
                ),*>;

            fn components_mask(world: &'b ::World<'a>) -> usize {
                $uo::components_mask(world) | $($u::components_mask(world)) | *
            }

            fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
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
                    marker: marker::PhantomData
                }
            }

            fn storage(world: &'b ::World<'a>) -> Self::Storage{
                $storage_ref{
                    $so: $uo::storage(world),
                    $(
                        $s: $u::storage(world),
                    )*
                }
            }

            fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
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

pub struct ReadAndParent<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteAndParent<'a, T: 'a + Component<'a>>{
    _marker: marker::PhantomData<&'a T>,
}


pub struct ParentStorageRead<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, (&'a T, Option<&'a T>)> for ParentStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a T, Option<&'a T>){
        let node = unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(self.storage.get_node(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&T, Option<&T>), (&T, Option<&T>)>((&node, parent)) }
    }
}

pub struct ReadAndParentIter<'a, T: Component<'a>>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component<'a>>::Storage>: IntoHierarchicalIter<'b,T>
{
    it: ForestHierarchicalIter<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for ReadAndParentIter<'a, T>
    where T: Component<'a>,
          <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockReadGuard<'b, <T as Component<'a>>::Storage>: IntoHierarchicalIter<'b,T>,
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


impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedData<'a, 'b> for ReadAndParent<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> RwLockReadGuard<'c, <T as Component<'a>>::Storage>: IntoHierarchicalIter<'c,T>,
          'a: 'b,
{
    type Iter = ReadAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a T, Option<&'a T>);
    type Storage = ParentStorageRead<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        ReadAndParentIter{
            it: world.storage::<T>().unwrap().into_hierarchical_iter()
        }
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        ParentStorageRead{
            storage: world.storage::<T>().unwrap(),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
        world.ordered_entities_for::<T>(mask)
    }
}



pub struct ParentStorageWrite<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<'a,T> + 'a, T: 'a + ComponentSync<'a>> StorageRef<'a, (&'a mut T, Option<&'a T>)> for ParentStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a mut T, Option<&'a T>){
        let mut node = unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>((*self.storage.get()).get_node_mut(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&mut T, Option<&T>), (&mut T, Option<&T>)>((&mut node, parent)) }
    }
}

pub struct WriteAndParentIter<'a, T: Component<'a>>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component<'a>>::Storage>: IntoHierarchicalIterMut<'b,T>
{
    it: ForestHierarchicalIterMut<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for WriteAndParentIter<'a, T>
    where T: Component<'a>,
          <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'b> RwLockWriteGuard<'b, <T as Component<'a>>::Storage>: IntoHierarchicalIterMut<'b,T>,
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

impl<'a, 'b, T: 'a + ComponentSync<'a>> OrderedData<'a, 'b> for WriteAndParent<'a,T>
    where <T as Component<'a>>::Storage: HierarchicalStorage<'a,T>,
          for<'c> RwLockWriteGuard<'c, <T as Component<'a>>::Storage>: IntoHierarchicalIterMut<'c,T>,
          'a: 'b,
{
    type Iter = WriteAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a mut T, Option<&'a T>);
    type Storage = ParentStorageWrite<'a, <T as Component<'a>>::Storage, Self::Components>;
    fn components_mask(world: &'b World<'a>) -> usize{
        world.components_mask::<T>()
    }

    fn into_iter(world: &'b ::World<'a>) -> Self::Iter{
        WriteAndParentIter{
            it: world.storage_mut::<T>().unwrap().into_hierarchical_iter_mut()
        }
    }

    fn storage(world: &'b ::World<'a>) -> Self::Storage{
        ParentStorageWrite{
            storage: UnsafeCell::new(world.storage_mut::<T>().unwrap()),
            _marker: marker::PhantomData,
        }
    }

    fn ordered_ids(world: &'b ::World<'a>, mask: usize) -> IndexGuard<'b>{
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
// impl<'a, T: 'a + OneToNComponentSync> UnorderedData<'a, 'b> for ReadOneToN<'a,T>
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
// impl<'a, T: 'a + OneToNComponentSync> UnorderedData<'a, 'b> for WriteOneToN<'a,T>
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
// impl<'a, T: 'a + OneToNComponentThreadLocal> UnorderedDataLocal<'a, 'b> for ReadOneToN<'a,T>
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
// impl<'a, T: 'a + OneToNComponentThreadLocal> UnorderedDataLocal<'a, 'b> for WriteOneToN<'a,T>
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
