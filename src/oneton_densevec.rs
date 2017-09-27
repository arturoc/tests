use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::marker;
use std::mem;
use std::slice;

use storage::{Storage, OneToNStorage, IntoIter, IntoIterMut};
use ::DenseVec;
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

struct Group{
    first_index: usize,
    len: usize,
}

pub struct DenseOneToNVec<T>{
    vec: Vec<T>,
    index: DenseVec<Group>,
    ids: Vec<usize>,
}

impl<'a,T: 'a> OneToNStorage<'a,T> for DenseOneToNVec<T>{
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
        let slice = &self.index.get(guid);
        &self.vec[slice.first_index..slice.first_index + slice.len]
    }

    unsafe fn get_slice_mut(&mut self, guid: usize) -> &mut [T]{
        let slice = &self.index.get(guid);
        &mut self.vec[slice.first_index..slice.first_index + slice.len]
    }
}



impl<'a, T: 'a> Storage<'a, T> for DenseOneToNVec<T>{
    type Get = &'a [T];
    type GetMut = &'a mut [T];

    fn new() -> DenseOneToNVec<T>{
        DenseOneToNVec{
            vec: vec![],
            index: DenseVec::new(),
            ids: vec![],
        }
    }

    fn with_capacity(capacity: usize) -> DenseOneToNVec<T>{
        DenseOneToNVec{
            vec: Vec::with_capacity(capacity),
            index: DenseVec::new(),
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
        let group = unsafe{ &self.index.get(guid) };
        self.vec.drain(group.first_index .. group.first_index + group.len);
        for i in group.first_index .. group.first_index + group.len{
            if let Some(i) = self.ids.iter().position(|id| *id == i){
                self.ids.remove(i);
            }
        }
    }

    unsafe fn get(&'a self, guid: usize) -> &'a [T]{
        let slice = &self.index.get(guid);
        &self.vec[slice.first_index..slice.first_index + slice.len]

        // let ptr = self.vec.as_ptr().offset(slice.first_index as isize);
        // slice::from_raw_parts(ptr, slice.len)
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut [T]{
        let slice = &self.index.get(guid);
        &mut self.vec[slice.first_index..slice.first_index + slice.len]

        // let ptr = self.vec.as_mut_ptr().offset(slice.first_index as isize);
        // slice::from_raw_parts_mut(ptr, slice.len)
    }
}

pub struct OneToNDenseIter<'a, T: 'a>{
    storage: ReadGuardRef<'a, DenseOneToNVec<T>>,
    it: slice::Iter<'a,Group>,
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
        let it = unsafe{ mem::transmute::<slice::Iter<Group>, slice::Iter<Group>>(self.index.iter()) };
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
    it: slice::IterMut<'a,Group>,
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
        let it = unsafe{ mem::transmute::<slice::IterMut<Group>, slice::IterMut<Group>>(self.index.iter_mut()) };
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
