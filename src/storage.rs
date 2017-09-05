use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::cell::UnsafeCell;
use std::mem;

use sync::{ReadGuardRef, WriteGuardRef};
use ::Component;
use ::ComponentSync;
use ::ComponentThreadLocal;
use ::World;

pub trait Storage<T>{
    fn new() -> Self;
    fn insert(&mut self, guid: usize, t: T);
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
// Sync Read/Write
pub struct Read<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

pub struct Write<'a, T: 'a + Component>{
    _marker: marker::PhantomData<&'a T>,
}

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
            next: usize,
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

                if self.next == self.ids.index.len(){
                    None
                }else{
                    let guid = self.ids.index[self.next];
                    self.next += 1;
                    Some(($(self.$s.get(guid)),*))
                }
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
                    $iter{
                        ids: world.entities_for_mask(Self::components_mask(world)),
                        next: 0,
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
                    $iter{
                        ids: world.entities_for_mask(Self::components_mask(world)),
                        next: 0,
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
