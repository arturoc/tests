use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::usize;

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};


use fnv::FnvHashMap;
use std::collections::hash_map::{Values, ValuesMut};
use std::mem;

pub struct HashMapStorage<T>{
    storage: FnvHashMap<usize, T>
}

impl<'a, T: 'a> Storage<'a, T> for HashMapStorage<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    fn new() -> HashMapStorage<T>{
        HashMapStorage{
            storage: FnvHashMap::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self{
        HashMapStorage{
            storage: FnvHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        self.storage.insert(guid, t);
    }

    fn remove(&mut self, guid: usize){
        self.storage.remove(&guid);
    }

    unsafe fn get(&'a self, guid: usize) -> &'a T{
        self.storage.get(&guid).unwrap()
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        self.storage.get_mut(&guid).unwrap()
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.contains_key(&guid)
    }
}

pub struct AssocIter<'a, T: 'a>{
    _guard: ReadGuardRef<'a, HashMapStorage<T>>,
    iter: Values<'a, usize, T>,
}

impl<'a, T: 'a> Iterator for AssocIter<'a, T>{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T>{
        self.iter.next()
    }
}

pub struct AssocIterMut<'a, T: 'a>{
    _guard: WriteGuardRef<'a, HashMapStorage<T>>,
    iter: ValuesMut<'a, usize, T>,
}

impl<'a, T: 'a> Iterator for AssocIterMut<'a, T>{
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T>{
        self.iter.next()
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, HashMapStorage<T>>{
    type Iter = AssocIter<'a, T>;
    fn into_iter(self) -> AssocIter<'a, T>{
        AssocIter{
            iter: unsafe{ mem::transmute::<Values<usize, T>, Values<usize,T>>(self.storage.values()) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, HashMapStorage<T>>{
    type Iter = AssocIter<'a, T>;
    fn into_iter(self) -> AssocIter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, HashMapStorage<T>>{
    type IterMut = AssocIterMut<'a, T>;
    fn into_iter_mut(mut self) -> AssocIterMut<'a, T>{
        AssocIterMut{
            iter: unsafe{ mem::transmute::<ValuesMut<usize, T>, ValuesMut<usize,T>>(self.storage.values_mut()) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, HashMapStorage<T>>{
    type IterMut = AssocIterMut<'a, T>;
    fn into_iter_mut(self) -> AssocIterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
