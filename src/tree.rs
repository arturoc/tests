use std::ptr;
use std::marker;
use std::mem;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use idtree;
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};
use storage::{Storage, IntoIter, IntoIterMut};

pub struct Tree<T>{
    arena: idtree::Arena<T>,
    index: Vec<idtree::NodeId>,
}

impl<T> Storage<T> for Tree<T>{
    fn new() -> Tree<T>{
        Tree{
            arena: idtree::Arena::new(),
            index: Vec::new(),
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        let node_id = self.arena.new_node(t);
        if self.index.capacity() < guid + 1{
            let diff = guid + 1 - self.index.len();
            self.index.reserve(diff);
        }
        if self.index.len() < guid +1 {
            unsafe{ self.index.set_len(guid+1) }
        }
        unsafe{ ptr::write(self.index.get_unchecked_mut(guid), node_id.id()) };
    }

    unsafe fn get(&self, guid: usize) -> &T{
        let node_id = self.index.get_unchecked(guid);
        &self.arena[*node_id]
    }

    unsafe fn get_mut(&mut self, guid: usize) -> &mut T{
        let node_id = self.index.get_unchecked(guid);
        &mut self.arena[*node_id]
    }
}



// pub struct Iter<'a, T: 'a>{
//     storage: ReadGuardRef<'a, Tree<T>>,
//     ids: &'a [usize],
//     next: usize,
//     _marker: marker::PhantomData<&'a T>,
// }
//
// impl<'a, T: 'a> Iterator for Iter<'a, T>{
//     type Item = &'a T;
//     fn next(&mut self) -> Option<&'a T>{
//         unsafe {
//             if self.next == self.ids.len() {
//                 None
//             } else {
//                 let ret = Some(mem::transmute::<&T,&T>(self.storage.get(*self.ids.get_unchecked(self.next))));
//                 self.next += 1;
//                 ret
//             }
//         }
//     }
// }
//
// pub struct IterMut<'a, T: 'a>{
//     storage: WriteGuardRef<'a, Tree<T>>,
//     ids: &'a [usize],
//     next: usize,
//     _marker: marker::PhantomData<&'a T>,
// }
//
// impl<'a, T: 'a> Iterator for IterMut<'a, T>{
//     type Item = &'a mut T;
//     fn next(&mut self) -> Option<&'a mut T>{
//         unsafe {
//             if self.next == self.ids.len() {
//                 None
//             } else {
//                 let ret = Some(mem::transmute::<&mut T,&mut T>(self.storage.get_mut(*self.ids.get_unchecked(self.next))));
//                 self.next += 1;
//                 ret
//             }
//         }
//     }
// }
//
//
// impl<'a, T> IntoIter for ReadGuardRef<'a, Tree<T>>{
//     type Iter = Iter<'a, T>;
//     fn into_iter(self) -> Iter<'a, T>{
//         Iter{
//             next: 0,
//             ids: unsafe{ mem::transmute::<&[usize], &[usize]>(self.ids.as_slice()) },
//             storage: self,
//             _marker: marker::PhantomData,
//         }
//     }
// }
//
// impl<'a, T> IntoIter for RwLockReadGuard<'a, Tree<T>>{
//     type Iter = Iter<'a, T>;
//     fn into_iter(self) -> Iter<'a, T>{
//         ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
//     }
// }
//
// impl<'a, T> IntoIterMut for WriteGuardRef<'a, Tree<T>>{
//     type IterMut = IterMut<'a, T>;
//     fn into_iter_mut(self) -> IterMut<'a, T>{
//         IterMut{
//             next: 0,
//             ids: unsafe{ mem::transmute::<&[usize], &[usize]>(self.ids.as_slice()) },
//             storage: self,
//             _marker: marker::PhantomData,
//         }
//     }
// }
//
// impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, Tree<T>>{
//     type IterMut = IterMut<'a, T>;
//     fn into_iter_mut(self) -> IterMut<'a, T>{
//         WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
//     }
// }
