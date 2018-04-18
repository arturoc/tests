use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::usize;
use std::mem;

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};
use densevec::DenseVec;

impl<'a, T: 'a> Storage<'a, T> for DenseVec<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    #[inline]
    fn new() -> DenseVec<T>{
        DenseVec::new()
    }

    #[inline]
    fn with_capacity(capacity: usize) -> Self{
        DenseVec::with_capacity(capacity)
    }

    #[inline]
    fn insert(&mut self, guid: usize, t: T){
        self.insert(guid, t);
    }

    #[inline]
    fn remove(&mut self, guid: usize){
        self.remove(guid).unwrap();
    }

    #[inline]
    unsafe fn get(&'a self, guid: usize) -> &'a T{
        self.get_unchecked(guid)
    }

    #[inline]
    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        self.get_unchecked_mut(guid)
    }

    #[inline]
    fn contains(&self, guid: usize) -> bool{
       self.contains_key(guid)
    }
}

pub struct DenseIter<'a, T: 'a>{
    _guard: ReadGuardRef<'a, DenseVec<T>>,
    iter: slice::Iter<'a, T>
}

impl<'a, T: 'a> Iterator for DenseIter<'a, T>{
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T>{
        self.iter.next()
    }
}

pub struct DenseIterMut<'a, T: 'a>{
    _guard: WriteGuardRef<'a, DenseVec<T>>,
    iter: slice::IterMut<'a, T>
}

impl<'a, T: 'a> Iterator for DenseIterMut<'a, T>{
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T>{
        self.iter.next()
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, DenseVec<T>>{
    type Iter = DenseIter<'a, T>;
    fn into_iter(self) -> DenseIter<'a, T>{
        DenseIter{
            iter: unsafe{ mem::transmute(self.values()) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, DenseVec<T>>{
    type Iter = DenseIter<'a, T>;
    fn into_iter(self) -> DenseIter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, DenseVec<T>>{
    type IterMut = DenseIterMut<'a, T>;
    fn into_iter_mut(mut self) -> DenseIterMut<'a, T>{
        DenseIterMut{
           iter: unsafe{ mem::transmute(self.values_mut()) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, DenseVec<T>>{
    type IterMut = DenseIterMut<'a, T>;
    fn into_iter_mut(self) -> DenseIterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
