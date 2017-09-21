use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::cell::UnsafeCell;
use std::mem;
use std::slice;

use sync::{ReadGuardRef, WriteGuardRef};
use ::Component;
use ::ComponentSync;
use ::ComponentThreadLocal;
use ::World;
use ::IndexGuard;
use ::Entity;

pub trait Storage<T>{
    fn new() -> Self;
    fn with_capacity(usize) -> Self;
    fn insert(&mut self, guid: usize, t: T);
    fn remove(&mut self, guid: usize);
    unsafe fn get(&self, guid: usize) -> &T;
    unsafe fn get_mut(&mut self, guid: usize) -> &mut T;
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

// Sync Read/Write
pub struct StorageRead<'a, S: Storage<T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWrite<'a, S: Storage<T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
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
        unsafe{ mem::transmute::<&mut T, &mut T>( (*self.storage.get()).get_mut(guid) ) }
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
    where for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <RwLockReadGuard<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a T;
    type Storage = StorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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
    type ComponentsRef = &'a mut T;
    type Storage = StorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

impl<'a> UnorderedData<'a> for ReadEntities {
    type Iter = slice::Iter<'a,Entity>;
    type Components = Entity;
    type ComponentsRef = &'a Entity;
    type Storage = &'a [Entity];
    fn components_mask(_world: &'a World) -> usize{
        0
    }

    fn into_iter(world: &'a ::World) -> Self::Iter{
        world.entities_ref().iter()
    }

    fn storage(world: &'a ::World) -> Self::Storage{
        world.entities_ref()
    }
}

// Thread local Read/Write
pub struct StorageReadLocal<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct StorageWriteLocal<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal> StorageRef<'a, &'a T> for StorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a T{
        unsafe{ mem::transmute::<&T, &T>(self.storage.get(guid)) }
    }
}

impl<'a, S: Storage<T> + 'a, T: 'a + ComponentThreadLocal> StorageRef<'a, &'a mut T> for StorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> &'a mut T{
        unsafe{ mem::transmute::<&mut T, &mut T>((*self.storage.get()).get_mut(guid)) }
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

impl<'a, T: 'a + ComponentThreadLocal> UnorderedDataLocal<'a> for Read<'a,T>
    where for<'b> ReadGuardRef<'b, <T as Component>::Storage>: IntoIter
{
    type Iter = <ReadGuardRef<'a, <T as Component>::Storage> as IntoIter>::Iter;
    type Components = T;
    type ComponentsRef = &'a T;
    type Storage = StorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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
    type ComponentsRef = &'a mut T;
    type Storage = StorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

            fn components_mask(world: &'a ::World) -> usize {
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

            fn components_mask(world: &'a ::World) -> usize {
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

pub trait HierarchicalStorage<T>: Storage<T>{
    unsafe fn insert_child(&mut self, parent_guid: usize, guid: usize, value: T);
    unsafe fn get_node(&self, guid: usize) -> idtree::NodeIdRef<T>;
    unsafe fn get_node_mut(&mut self, guid: usize) -> idtree::NodeIdMut<T>;
    fn ordered_ids(&self) -> &[usize];
}

pub struct ReadHierarchical<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct WriteHierarchical<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}


// Sync Hierarchical Read/Write
pub struct HierarchicalStorageRead<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWrite<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeIdRef<'a, T>> for HierarchicalStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeIdRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeIdRef<T>, idtree::NodeIdRef<T>>(self.storage.get_node(guid)) }
    }
}

impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeIdMut<'a, T>> for HierarchicalStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeIdMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeIdMut<T>, idtree::NodeIdMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }
}


pub trait OrderedData<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> usize;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard;
}


impl<'a, T: 'a + ComponentSync> OrderedData<'a> for ReadHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoOrderedIter
{
    type Iter = <RwLockReadGuard<'a, <T as Component>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeIdRef<'a, T>;
    type Storage = HierarchicalStorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}


impl<'a, T: 'a + ComponentSync> OrderedData<'a> for WriteHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoOrderedIterMut
{
    type Iter = <RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeIdMut<'a, T>;
    type Storage = HierarchicalStorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}

// Local Hierarchical Read/Write
pub struct HierarchicalStorageReadLocal<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: ReadGuardRef<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

pub struct HierarchicalStorageWriteLocal<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentThreadLocal>{
    storage: UnsafeCell<WriteGuardRef<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}


impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeIdRef<'a, T>> for HierarchicalStorageReadLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeIdRef<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeIdRef<T>, idtree::NodeIdRef<T>>(self.storage.get_node(guid)) }
    }
}

impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, idtree::NodeIdMut<'a, T>> for HierarchicalStorageWriteLocal<'a, S, T>{
    fn get(&self, guid: usize) -> idtree::NodeIdMut<'a, T>{
        unsafe{ mem::transmute::<idtree::NodeIdMut<T>, idtree::NodeIdMut<T>>((*self.storage.get()).get_node_mut(guid)) }
    }
}


pub trait OrderedDataLocal<'a>{
    type Iter;
    type Components: 'a;
    type ComponentsRef;
    type Storage;
    fn components_mask(world: &'a World) -> usize;
    fn into_iter(world: &'a ::World) -> Self::Iter;
    fn storage(world: &'a ::World) -> Self::Storage;
    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard;
}


impl<'a, T: 'a + ComponentSync> OrderedDataLocal<'a> for ReadHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> ReadGuardRef<'b, <T as Component>::Storage>: IntoOrderedIter
{
    type Iter = <ReadGuardRef<'a, <T as Component>::Storage> as IntoOrderedIter>::OrderedIter;
    type Components = T;
    type ComponentsRef = idtree::NodeIdRef<'a, T>;
    type Storage = HierarchicalStorageReadLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
        world.thread_local_ordered_entities_for::<T>(mask)
    }
}


impl<'a, T: 'a + ComponentSync> OrderedDataLocal<'a> for WriteHierarchical<'a,T>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> WriteGuardRef<'b, <T as Component>::Storage>: IntoOrderedIterMut
{
    type Iter = <WriteGuardRef<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut;
    type Components = T;
    type ComponentsRef = idtree::NodeIdMut<'a, T>;
    type Storage = HierarchicalStorageWriteLocal<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
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

            fn components_mask(world: &'a ::World) -> usize {
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

            fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
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

            fn components_mask(world: &'a ::World) -> usize {
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

            fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
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


pub struct ParentStorageRead<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync>{
    storage: RwLockReadGuard<'a, S>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, (&'a T, Option<&'a T>)> for ParentStorageRead<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a T, Option<&'a T>){
        let node = unsafe{ mem::transmute::<idtree::NodeIdRef<T>, idtree::NodeIdRef<T>>(self.storage.get_node(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&T, Option<&T>), (&T, Option<&T>)>((&node, parent)) }
    }
}

pub struct ReadAndParentIter<'a, T: Component>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoHierarchicalIter<'b,T>
{
    it: ForestHierarchicalIter<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for ReadAndParentIter<'a, T>
    where T: Component,
          <T as Component>::Storage: HierarchicalStorage<T>,
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
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockReadGuard<'b, <T as Component>::Storage>: IntoHierarchicalIter<'b,T>
{
    type Iter = ReadAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a T, Option<&'a T>);
    type Storage = ParentStorageRead<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}



pub struct ParentStorageWrite<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync>{
    storage: UnsafeCell<RwLockWriteGuard<'a, S>>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, S: HierarchicalStorage<T> + 'a, T: 'a + ComponentSync> StorageRef<'a, (&'a mut T, Option<&'a T>)> for ParentStorageWrite<'a, S, T>{
    fn get(&self, guid: usize) -> (&'a mut T, Option<&'a T>){
        let mut node = unsafe{ mem::transmute::<idtree::NodeIdMut<T>, idtree::NodeIdMut<T>>((*self.storage.get()).get_node_mut(guid)) };
        let parent = node.parent().map(|p| unsafe{ mem::transmute::<&T, &T>(&p) });
        unsafe{ mem::transmute::<(&mut T, Option<&T>), (&mut T, Option<&T>)>((&mut node, parent)) }
    }
}

pub struct WriteAndParentIter<'a, T: Component>
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoHierarchicalIterMut<'b,T>
{
    it: ForestHierarchicalIterMut<'a, T> //<RwLockWriteGuard<'a, <T as Component>::Storage> as IntoOrderedIterMut>::OrderedIterMut
}

impl<'a,T> Iterator for WriteAndParentIter<'a, T>
    where T: Component,
          <T as Component>::Storage: HierarchicalStorage<T>,
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
    where <T as Component>::Storage: HierarchicalStorage<T>,
          for<'b> RwLockWriteGuard<'b, <T as Component>::Storage>: IntoHierarchicalIterMut<'b,T>
{
    type Iter = WriteAndParentIter<'a,T>;
    type Components = T;
    type ComponentsRef = (&'a mut T, Option<&'a T>);
    type Storage = ParentStorageWrite<'a, <T as Component>::Storage, Self::Components>;
    fn components_mask(world: &'a World) -> usize{
        world.components_mask::<T>()
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

    fn ordered_ids(world: &'a ::World, mask: usize) -> IndexGuard{
        world.ordered_entities_for::<T>(mask)
    }
}


// OneToN
// use component::{OneToNComponent, OneToNComponentSync, OneToNComponentThreadLocal};

pub trait OneToNStorage<T>: Storage<T>{
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
