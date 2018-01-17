
use std::marker;
use std::ptr;
use std::mem;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

pub struct VecStorage<T>{
    storage: Vec<T>,
    ids: Vec<usize>,
}

impl<'a,T: 'a> Storage<'a,T> for VecStorage<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    fn new() -> VecStorage<T>{
        VecStorage{
            storage: vec![],
            ids: vec![],
        }
    }

    fn with_capacity(capacity: usize) -> VecStorage<T>{
        VecStorage{
            storage: Vec::with_capacity(capacity),
            ids: Vec::with_capacity(capacity),
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        if self.storage.capacity() < guid + 1{
            let diff = guid + 1 - self.storage.len();
            self.storage.reserve(diff);
        }
        if self.storage.len() < guid +1 {
            unsafe{ self.storage.set_len(guid+1) }
        }
        unsafe{ ptr::write(self.storage.get_unchecked_mut(guid), t) };
        self.ids.push(guid);
    }

    fn remove(&mut self, guid: usize){
        unsafe{ mem::replace(self.storage.get_unchecked_mut(guid), mem::uninitialized()) };
        if let Some(pos) = self.ids.iter().position(|id| *id == guid){
            self.ids.remove(pos);
        }
    }

    unsafe fn get(&'a self, guid: usize) -> &'a T{
        self.storage.get_unchecked(guid)
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        self.storage.get_unchecked_mut(guid)
    }

    //FIXME: This is super slow for bigger collections, use binary search to store sorted ids?
    fn contains(&self, guid: usize) -> bool{
        self.ids.contains(&guid)
    }
}


pub struct Iter<'a, T: 'a>{
    storage: ReadGuardRef<'a, VecStorage<T>>,
    ids: &'a [usize],
    next: usize,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T>{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T>{
        unsafe {
            if self.next == self.ids.len() {
                None
            } else {
                let ret = Some(mem::transmute::<&T,&T>(self.storage.get(*self.ids.get_unchecked(self.next))));
                self.next += 1;
                ret
            }
        }
    }
}

pub struct IterMut<'a, T: 'a>{
    storage: WriteGuardRef<'a, VecStorage<T>>,
    ids: &'a [usize],
    next: usize,
    _marker: marker::PhantomData<&'a T>,
}

impl<'a, T: 'a> Iterator for IterMut<'a, T>{
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T>{
        unsafe {
            if self.next == self.ids.len() {
                None
            } else {
                let ret = Some(mem::transmute::<&mut T,&mut T>(self.storage.get_mut(*self.ids.get_unchecked(self.next))));
                self.next += 1;
                ret
            }
        }
    }
}


impl<'a, T> IntoIter for ReadGuardRef<'a, VecStorage<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        Iter{
            next: 0,
            ids: unsafe{ mem::transmute::<&[usize], &[usize]>(self.ids.as_slice()) },
            storage: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, VecStorage<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, VecStorage<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(self) -> IterMut<'a, T>{
        IterMut{
            next: 0,
            ids: unsafe{ mem::transmute::<&[usize], &[usize]>(self.ids.as_slice()) },
            storage: self,
            _marker: marker::PhantomData,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, VecStorage<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(self) -> IterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
