
use std::marker;
use std::ptr;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use storage::{Storage, IntoIter, IntoIterMut};
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};

pub struct DenseVec<T>{
    storage: Vec<T>,
    index: Vec<usize>,
}

impl<T> Storage<T> for DenseVec<T>{
    fn new() -> DenseVec<T>{
        DenseVec{
            storage: vec![],
            index: vec![],
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        let id = self.storage.len();
        self.storage.push(t);
        if self.index.capacity() < guid + 1{
            let diff = guid + 1 - self.index.len();
            self.index.reserve(diff);
        }
        if self.index.len() < guid +1 {
            unsafe{ self.index.set_len(guid+1) }
        }
        unsafe{ ptr::write(self.index.get_unchecked_mut(guid), id) };
    }

    unsafe fn get(&self, guid: usize) -> &T{
        self.storage.get_unchecked(self.index[guid])
    }

    unsafe fn get_mut(&mut self, guid: usize) -> &mut T{
        self.storage.get_unchecked_mut(self.index[guid])
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
