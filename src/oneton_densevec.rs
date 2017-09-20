use std::collections::HashMap;
use std::collections::hash_map::{Values, ValuesMut};
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::mem;

use storage::{Storage, OneToNStorage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

struct Group{
    first_index: usize,
    len: usize,
}

pub struct DenseOneToNVec<T>{
    vec: Vec<T>,
    index: HashMap<usize,Group>,
    ids: Vec<usize>,
}

impl<T> OneToNStorage<T> for DenseOneToNVec<T>{
    fn insert_slice(&mut self, guid: usize, t: &[T]) where T: Clone{
        let group = self.index.entry(guid)
            .or_insert(Group{first_index: self.vec.len(), len: 0});
        let prev_len = self.vec.len();
        let rest = self.vec.split_off(group.first_index + group.len);
        self.vec.extend_from_slice(t);
        self.vec.extend(rest);
        self.ids.extend(prev_len..self.vec.len());
        group.len += t.len();
    }

    unsafe fn get_slice(&self, guid: usize) -> &[T]{
        let slice = &self.index[&guid];
        &self.vec[slice.first_index..slice.first_index + slice.len]
    }

    unsafe fn get_slice_mut(&mut self, guid: usize) -> &mut [T]{
        let slice = &self.index[&guid];
        &mut self.vec[slice.first_index..slice.first_index + slice.len]
    }
}



impl<T> Storage<T> for DenseOneToNVec<T>{
    fn new() -> DenseOneToNVec<T>{
        DenseOneToNVec{
            vec: vec![],
            index: HashMap::new(),
            ids: vec![],
        }
    }

    fn with_capacity(capacity: usize) -> DenseOneToNVec<T>{
        DenseOneToNVec{
            vec: Vec::with_capacity(capacity),
            index: HashMap::new(),
            ids: vec![],
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        let group = self.index.entry(guid)
            .or_insert(Group{first_index: self.vec.len(), len: 0});
        self.vec.insert(group.first_index + group.len, t);
        group.len += 1;
        self.ids.push(self.vec.len() - 1);
    }

    fn remove(&mut self, guid: usize){
        let group = &self.index[&guid];
        self.vec.drain(group.first_index .. group.first_index + group.len);
        for i in group.first_index .. group.first_index + group.len{
            if let Some(i) = self.ids.iter().position(|id| *id == i){
                self.ids.remove(i);
            }
        }
    }

    unsafe fn get(&self, guid: usize) -> &T{
        self.get_slice(guid).get_unchecked(0)
    }

    unsafe fn get_mut(&mut self, guid: usize) -> &mut T{
        self.get_slice_mut(guid).get_unchecked_mut(0)
    }
}


pub struct OneToNDenseIter<'a, T: 'a>{
    storage: ReadGuardRef<'a, DenseOneToNVec<T>>,
    it: Values<'a,usize,Group>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for OneToNDenseIter<'a, T>{
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]>{
        self.it.next().map(|group| {
            let v = &self.storage.vec[group.first_index..group.first_index + group.len];
            unsafe{mem::transmute::<&[T], &[T]>(v)}
        })
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, DenseOneToNVec<T>>{
    type Iter = OneToNDenseIter<'a, T>;
    fn into_iter(self) -> OneToNDenseIter<'a, T>{
        let it = unsafe{ mem::transmute::<Values<usize,Group>, Values<usize,Group>>(self.index.values()) };
        OneToNDenseIter{
            it,
            storage: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, DenseOneToNVec<T>>{
    type Iter = OneToNDenseIter<'a, T>;
    fn into_iter(self) -> OneToNDenseIter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}



pub struct OneToNDenseIterMut<'a, T: 'a>{
    storage: WriteGuardRef<'a, DenseOneToNVec<T>>,
    it: ValuesMut<'a,usize,Group>,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for OneToNDenseIterMut<'a, T>{
    type Item = &'a mut [T];
    fn next(&mut self) -> Option<&'a mut [T]>{
        self.it.next().map(|group| {
            let v = &mut self.storage.vec[group.first_index..group.first_index + group.len];
            unsafe{mem::transmute::<&mut [T], &mut [T]>(v)}
        })
    }
}


impl<'a, T> IntoIterMut for WriteGuardRef<'a, DenseOneToNVec<T>>{
    type IterMut = OneToNDenseIterMut<'a, T>;
    fn into_iter_mut(mut self) -> OneToNDenseIterMut<'a, T>{
        let it = unsafe{ mem::transmute::<ValuesMut<usize,Group>, ValuesMut<usize,Group>>(self.index.values_mut()) };
        OneToNDenseIterMut{
            it,
            storage: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, DenseOneToNVec<T>>{
    type IterMut = OneToNDenseIterMut<'a, T>;
    fn into_iter_mut(self) -> OneToNDenseIterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
