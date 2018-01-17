use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::usize;
use std::cell::UnsafeCell;
use std::slice;

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

pub struct AssocVec<T>{
    storage: Vec<(usize, T)>,
    last_returned: UnsafeCell<usize>,
    last_range: UnsafeCell<usize>,
    threshold: usize,
}

impl<'a, T: 'a> Storage<'a, T> for AssocVec<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    fn new() -> AssocVec<T>{
        AssocVec{
            storage: vec![],
            last_returned: UnsafeCell::new(0),
            last_range: UnsafeCell::new(0),
            threshold: 0,
        }
    }

    fn with_capacity(capacity: usize) -> Self{
        AssocVec{
            storage: Vec::with_capacity(capacity),
            last_returned: UnsafeCell::new(0),
            last_range: UnsafeCell::new(0),
            threshold: 0,
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
            Ok(_) => panic!("Trying to insert already exisiting compoenent"),
            Err(pos) => {
                self.storage.insert(pos, (pos, t));
                self.threshold = (self.storage.len() as f32).log2() as usize;
            }
        }
    }

    fn remove(&mut self, guid: usize){
        match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
            Ok(pos) => {
                self.storage.remove(pos);
                self.threshold = (self.storage.len() as f32).log2() as usize;
            },
            Err(_) => panic!("Element doesn't exist")
        }
    }

    fn contains(&self, guid: usize) -> bool{
        self.storage.binary_search_by_key(&guid, |&(id, _)| id).is_ok()
    }

    unsafe fn get(&'a self, guid: usize) -> &'a T{
        // let pos = match self.storage[*self.last_returned.get()..].binary_search_by_key(&guid, |&(id, _)| id){
        //     Ok(pos) => pos,
        //     Err(_) => match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
        //         Ok(pos) => pos,
        //         Err(_) => panic!("Element doesn't exist")
        //     }
        // };

        let pos = if *self.last_range.get() > self.threshold {
            let end = (*self.last_returned.get() + *self.last_range.get()).min(self.storage.len());
            match self.storage[*self.last_returned.get()..end].binary_search_by_key(&guid, |&(id, _)| id){
                Ok(pos) => pos,
                Err(_) => match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
                    Ok(pos) => pos,
                    Err(_) => panic!("Element doesn't exist")
                }
            }
        }else{
            match self.storage[*self.last_returned.get()..].iter().position(|&(id, _)| id == guid){
                Some(pos) => *self.last_returned.get() + pos,
                None => match self.storage.iter().position(|&(id, _)| id == guid){
                    Some(pos) => pos,
                    None => panic!("Element doesn't exist")
                }
            }
        };

        *self.last_range.get() = if *self.last_returned.get() < pos { pos - *self.last_returned.get() + 1 } else { pos + 1 };
        *self.last_returned.get() = pos;
        &self.storage.get_unchecked(pos).1
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        // let pos = match self.storage[*self.last_returned.get()..].binary_search_by_key(&guid, |&(id, _)| id){
        //     Ok(pos) => pos,
        //     Err(_) => match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
        //         Ok(pos) => pos,
        //         Err(_) => panic!("Element doesn't exist")
        //     }
        // };

        let pos = if *self.last_range.get() > self.threshold {
            let end = (*self.last_returned.get() + *self.last_range.get()).min(self.storage.len());
            match self.storage[*self.last_returned.get()..end].binary_search_by_key(&guid, |&(id, _)| id){
                Ok(pos) => pos,
                Err(_) => match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
                    Ok(pos) => pos,
                    Err(_) => panic!("Element doesn't exist")
                }
            }
        }else{
            match self.storage[*self.last_returned.get()..].iter().position(|&(id, _)| id == guid){
                Some(pos) => *self.last_returned.get() + pos,
                None => match self.storage.iter().position(|&(id, _)| id == guid){
                    Some(pos) => pos,
                    None => panic!("Element doesn't exist")
                }
            }
        };


        *self.last_range.get() = if *self.last_returned.get() < pos { pos - *self.last_returned.get() + 1 } else { pos };
        *self.last_returned.get() = pos;
        &mut self.storage.get_unchecked_mut(pos).1
    }
}

pub struct OccupiedEntry<'a, T: 'a>(&'a mut T);

pub struct VacantEntry<'a, T: 'a>{
    storage: &'a mut AssocVec<T>,
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
}

impl<T> AssocVec<T>{
    pub fn len(&self) -> usize{
        self.storage.len()
    }

    pub fn iter(&self) -> slice::Iter<(usize, T)>{
        self.storage.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<(usize, T)>{
        self.storage.iter_mut()
    }

    pub fn entry(&mut self, guid: usize) -> Entry<T>{
        if guid >= self.len(){
            Entry::Vacant(VacantEntry{storage: self, guid})
        }else{
            match self.storage.binary_search_by_key(&guid, |&(id, _)| id){
                Ok(pos) => {
                    Entry::Occupied(OccupiedEntry(unsafe{ &mut self.storage.get_unchecked_mut(pos).1 }))
                },
                Err(_) => Entry::Vacant(VacantEntry{storage: self, guid})
            }
        }
    }
}

pub struct AssocIter<'a, T: 'a>{
    _guard: ReadGuardRef<'a, AssocVec<T>>,
    ptr: *const (usize, T),
    end: *const (usize, T),
}

impl<'a, T: 'a> Iterator for AssocIter<'a, T>{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T>{
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                let ret = Some(&(*self.ptr).1);
                self.ptr = self.ptr.offset(1);
                ret
            }
        }
    }
}

pub struct AssocIterMut<'a, T: 'a>{
    _guard: WriteGuardRef<'a, AssocVec<T>>,
    ptr: *mut (usize, T),
    end: *mut (usize, T),
}

impl<'a, T: 'a> Iterator for AssocIterMut<'a, T>{
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T>{
        unsafe {
            if self.ptr == self.end {
                None
            } else {
                let ret = Some(&mut (*self.ptr).1);
                self.ptr = self.ptr.offset(1);
                ret
            }
        }
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, AssocVec<T>>{
    type Iter = AssocIter<'a, T>;
    fn into_iter(self) -> AssocIter<'a, T>{
        AssocIter{
            ptr: self.storage.as_ptr(),
            end: unsafe{ self.storage.as_ptr().offset(self.storage.len() as isize) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, AssocVec<T>>{
    type Iter = AssocIter<'a, T>;
    fn into_iter(self) -> AssocIter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, AssocVec<T>>{
    type IterMut = AssocIterMut<'a, T>;
    fn into_iter_mut(mut self) -> AssocIterMut<'a, T>{
        AssocIterMut{
            ptr: self.storage.as_mut_ptr(),
            end: unsafe{ self.storage.as_mut_ptr().offset(self.storage.len() as isize) },
            _guard: self,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, AssocVec<T>>{
    type IterMut = AssocIterMut<'a, T>;
    fn into_iter_mut(self) -> AssocIterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
