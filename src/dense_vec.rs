
use std::marker;
use std::ptr;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::slice;
use std::usize;

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

#[derive(Clone)]
pub struct DenseVec<T>{
    storage: Vec<T>,
    index: Vec<usize>,
    len: usize,
}

impl<'a, T: 'a> Storage<'a, T> for DenseVec<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    fn new() -> DenseVec<T>{
        DenseVec{
            storage: vec![],
            index: vec![],
            len: 0,
        }
    }

    fn with_capacity(capacity: usize) -> Self{
        DenseVec{
            storage: Vec::with_capacity(capacity),
            index: Vec::with_capacity(capacity),
            len: 0,
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        let id = self.storage.len();
        self.storage.push(t);
        if self.index.len() < guid + 1{
            // let diff = guid + 1 - self.index.len();
            // self.index.reserve(diff);
            self.index.resize(guid + 1, usize::MAX)
        }
        // if self.index.len() < guid +1 {
        //     unsafe{ self.index.set_len(guid+1) }
        // }
        unsafe{ ptr::write(self.index.get_unchecked_mut(guid), id) };
        self.len += 1;
    }

    fn remove(&mut self, guid: usize){
        let index = unsafe{ *self.index.get_unchecked(guid) };
        self.storage.remove(index);
        for i in self.index.iter_mut().filter(|i| **i > index){
            *i -= 1;
        }
        self.len -= 1;
    }

    unsafe fn get(&'a self, guid: usize) -> &'a T{
        let idx = *self.index.get_unchecked(guid);
        self.storage.get_unchecked(idx)
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        self.storage.get_unchecked_mut(*self.index.get_unchecked(guid))
    }
}

pub struct OccupiedEntry<'a, T: 'a>(&'a mut T);

pub struct VacantEntry<'a, T: 'a>{
    storage: &'a mut DenseVec<T>,
    guid: usize,
}

pub enum Entry<'a, T: 'a>{
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, T>),
}

impl<'a, T: 'a> Entry<'a, T>{
    pub fn or_insert(self, default: T) -> &'a mut T{
        match self{
            Entry::Occupied(OccupiedEntry(t)) => t,
            Entry::Vacant(VacantEntry{storage, guid}) => {
                storage.insert(guid, default);
                unsafe{ storage.get_mut(guid) }
            }
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut T
        where F: FnOnce() -> T
    {
        match self{
            Entry::Occupied(OccupiedEntry(t)) => t,
            Entry::Vacant(VacantEntry{storage, guid}) => {
                storage.insert(guid, default());
                unsafe{ storage.get_mut(guid) }
            }
        }
    }
}

impl<T> DenseVec<T>{
    pub fn len(&self) -> usize{
        self.len
    }

    pub fn iter(&self) -> slice::Iter<T>{
        self.storage.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<T>{
        self.storage.iter_mut()
    }

    pub fn entry(&mut self, guid: usize) -> Entry<T>{
        if guid >= self.index.len() {
            Entry::Vacant(VacantEntry{storage: self, guid})
        }else{
            let idx = unsafe{ *self.index.get_unchecked(guid) };
            if idx == usize::MAX {
                Entry::Vacant(VacantEntry{storage: self, guid})
            }else{
                Entry::Occupied(OccupiedEntry(unsafe{ self.storage.get_unchecked_mut(idx) }))
            }
        }
    }

    pub fn contains(&self, guid: usize) -> bool{
        guid < self.index.len() && unsafe{ *self.index.get_unchecked(guid) } < usize::MAX
    }
}

impl<T> IntoIterator for DenseVec<T>{
    type Item = T;
    type IntoIter = ::std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter{
        self.storage.into_iter()
    }
}


pub struct DenseIter<'a, T: 'a>{
    _guard: ReadGuardRef<'a, DenseVec<T>>,
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
    _guard: WriteGuardRef<'a, DenseVec<T>>,
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


impl<'a, T> IntoIter for ReadGuardRef<'a, DenseVec<T>>{
    type Iter = DenseIter<'a, T>;
    fn into_iter(self) -> DenseIter<'a, T>{
        DenseIter{
            ptr: self.storage.as_ptr(),
            end: unsafe{ self.storage.as_ptr().offset(self.storage.len() as isize) },
            _guard: self,
            _marker: marker::PhantomData,
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
            ptr: self.storage.as_mut_ptr(),
            end: unsafe{ self.storage.as_mut_ptr().offset(self.storage.len() as isize) },
            _guard: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, DenseVec<T>>{
    type IterMut = DenseIterMut<'a, T>;
    fn into_iter_mut(self) -> DenseIterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}

// pub struct DenseUnorderedIter<'a, T: 'a>{
//     storage: RwLockReadGuard<'a, DenseVec<T>>,
//     ids: &'a [usize],
//     next: usize,
// }
//
// impl<'a, T: 'a> Iterator for DenseUnorderedIter<'a, T>{
//     type Item = &'a T;
//     fn next(&mut self) -> Option<&'a T>{
//         if self.next == self.ids.len(){
//             None
//         }else{
//             let ret = Some(unsafe{ mem::transmute::<&T,&T>(self.storage.get(self.ids[self.next])) });
//             self.next += 1;
//             ret
//         }
//     }
// }
