use std::ptr;
use std::mem;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::cell::UnsafeCell;

use idtree;
use sync::{ReadGuardRef, WriteGuardRef, ReadGuard, WriteGuard};
use storage::{Storage, IntoIter, IntoIterMut, HierarchicalStorage, IntoOrderedIter, IntoOrderedIterMut};

pub struct Forest<T>{
    arena: idtree::Arena<T>,
    roots: Vec<idtree::NodeId>,
    index: Vec<idtree::NodeId>,
    ordered_ids: UnsafeCell<Vec<usize>>,
}

impl<'a, T: 'a> Storage<'a, T> for Forest<T>{
    type Get = &'a T;
    type GetMut = &'a mut T;

    fn new() -> Forest<T>{
        Forest{
            arena: idtree::Arena::new(),
            roots: Vec::new(),
            index: Vec::new(),
            ordered_ids: UnsafeCell::new(vec![]),
        }
    }

    fn with_capacity(capacity: usize) -> Forest<T>{
        Forest{
            arena: idtree::Arena::with_capacity(capacity),
            roots: Vec::with_capacity(capacity),
            index: Vec::with_capacity(capacity),
            ordered_ids: UnsafeCell::new(Vec::with_capacity(capacity)),
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
        self.roots.push(node_id.id());
    }

    fn remove(&mut self, guid: usize){
        let id = unsafe{ self.index.get_unchecked(guid) };
        self.arena.remove(*id);
        if let Some(pos) = self.roots.iter().position(|i| i == id){
            self.roots.remove(pos);
        }
        unsafe{ (*self.ordered_ids.get()).clear() };
    }

    unsafe fn get(&'a self, guid: usize) -> &'a T{
        let node_id = self.index.get_unchecked(guid);
        &self.arena[*node_id]
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> &'a mut T{
        let node_id = self.index.get_unchecked(guid);
        &mut self.arena[*node_id]
    }
}

pub struct Iter<'a, T: 'a>{
    _guard: ReadGuardRef<'a, Forest<T>>,
    it: idtree::AllNodes<'a, T>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T>{
    type Item = &'a T;
    #[inline]
    fn next(&mut self) -> Option<&'a T>{
        self.it.next().map(|node| &node.data)
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, Forest<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        Iter{
            it: unsafe{mem::transmute::<idtree::AllNodes<T>, idtree::AllNodes<T>>(self.arena.all_nodes())},
            _guard: self,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, Forest<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}

pub struct IterMut<'a, T: 'a>{
    _guard: WriteGuardRef<'a, Forest<T>>,
    it: idtree::AllNodesMut<'a, T>
}

impl<'a, T: 'a> Iterator for IterMut<'a, T>{
    type Item = &'a mut T;
    #[inline]
    fn next(&mut self) -> Option<&'a mut T>{
        self.it.next().map(|node| &mut node.data)
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, Forest<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(mut self) -> IterMut<'a, T>{
        IterMut{
            it: unsafe{mem::transmute::<idtree::AllNodesMut<T>, idtree::AllNodesMut<T>>(self.arena.all_nodes_mut())},
            _guard: self,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, Forest<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(self) -> IterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}


impl<'a,T: 'a> HierarchicalStorage<'a,T> for Forest<T>{
    unsafe fn insert_child(&mut self, parent_guid: usize, guid: usize, value: T){
        let parent_id = *self.index.get_unchecked(parent_guid);
        let node_id = self.arena.get_mut(parent_id).append_new(value);
        if self.index.capacity() < guid + 1{
            let diff = guid + 1 - self.index.len();
            self.index.reserve(diff);
        }
        if self.index.len() < guid +1 {
            self.index.set_len(guid+1)
        }
        ptr::write(self.index.get_unchecked_mut(guid), node_id.id());
    }

    unsafe fn get_node(&self, guid: usize) -> idtree::NodeRef<T>{
        let node_id = *self.index.get_unchecked(guid);
        self.arena.get(node_id)
    }

    unsafe fn get_node_mut(&mut self, guid: usize) -> idtree::NodeRefMut<T>{
        let node_id = self.index.get_unchecked(guid);
        self.arena.get_mut(*node_id)
    }

    fn ordered_ids(&self) -> &[usize]{
        if unsafe{(*self.ordered_ids.get()).is_empty()}{
            let iter_tree = ForestHierarchicalIdsIter{
                forest: self,
                current: if self.roots.is_empty(){ None } else { Some(0) },
                iter: if self.roots.is_empty(){ None } else { Some(self.roots[0].descendants(&self.arena)) }
            };
            unsafe{ (*self.ordered_ids.get()).extend(iter_tree
                .map(|id| self.index.iter().position(|id2| *id2 == id).unwrap()))};
        }
        unsafe{ &*(*self.ordered_ids.get()) }
    }
}

impl<'a, T> IntoOrderedIter for ReadGuardRef<'a, Forest<T>>{
    type OrderedIter = ForestHierarchicalIter<'a,T>;
    fn into_ordered_iter(self) -> Self::OrderedIter{
        self.ordered_ids();
        ForestHierarchicalIter{
            // current: if self.roots.is_empty(){ None } else { Some(0) },
            //iter: if self.roots.is_empty(){ None } else { Some(self.roots[0].descendants(unsafe{mem::transmute::<&idtree::Arena<T>, &idtree::Arena<T>>(&self.arena)})) },
            forest: self,
            next: 0,
        }
    }
}

impl<'a, T> IntoOrderedIter for RwLockReadGuard<'a, Forest<T>>{
    type OrderedIter = ForestHierarchicalIter<'a,T>;
    fn into_ordered_iter(self) -> Self::OrderedIter{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_ordered_iter()
    }
}

struct ForestHierarchicalIdsIter<'a, T: 'a>{
    forest: &'a Forest<T>,
    current: Option<usize>,
    iter: Option<idtree::Descendants<'a, T>>,
}

impl<'a, T: 'a> Iterator for ForestHierarchicalIdsIter<'a, T>{
    type Item = idtree::NodeId;
    fn next(&mut self) -> Option<idtree::NodeId>{
        let iter = if let Some(ref mut iter) = self.iter{
            let next = iter.next();
            if let Some(next) = next{
                return Some(self.forest.arena.get(next).id());
            }else{
                if let Some(current) = self.current{
                    if current + 1 < self.forest.roots.len(){
                        self.current = Some(current + 1);
                        Some(self.forest.roots[current + 1].descendants(unsafe{mem::transmute::<&idtree::Arena<T>, &idtree::Arena<T>>(&self.forest.arena)}))
                    }else{
                        self.current = None;
                        None
                    }
                }else{
                    None
                }
            }
        }else{
            None
        };

        if let Some(mut iter) = iter{
            let next = iter.next().map(|id| self.forest.arena.get(id).id() );
            self.iter = Some(iter);
            next
        }else{
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>){
        (self.forest.arena.len(), Some(self.forest.arena.len()))
    }
}

pub struct ForestHierarchicalIter<'a, T: 'a>{
    forest: ReadGuardRef<'a, Forest<T>>,
    // current: Option<usize>,
    next: usize,
    //iter: Option<idtree::Descendants<'a, T>>,
}

impl<'a, T: 'a> Iterator for ForestHierarchicalIter<'a, T>{
    type Item = idtree::NodeRef<'a,T>;
    fn next(&mut self) -> Option<idtree::NodeRef<'a,T>>{
        if self.next == unsafe{(*self.forest.ordered_ids.get()).len()}{
            None
        }else{
            let next = unsafe{ *(*self.forest.ordered_ids.get()).get_unchecked(self.next) };
            let node = unsafe{ self.forest.get_node(next) };
            self.next += 1;
            let node = unsafe{ mem::transmute::<idtree::NodeRef<T>, idtree::NodeRef<T>>(node) };
            Some(node)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>){
        (self.forest.arena.len(), Some(self.forest.arena.len()))
    }
}

impl<'a, T> IntoOrderedIterMut for WriteGuardRef<'a, Forest<T>>{
    type OrderedIterMut = ForestHierarchicalIterMut<'a,T>;
    fn into_ordered_iter_mut(self) -> Self::OrderedIterMut{
        self.ordered_ids();
        ForestHierarchicalIterMut{
            // current: if self.roots.is_empty(){ None } else { Some(0) },
            // iter: if self.roots.is_empty(){ None } else { Some(self.roots[0].descendants(unsafe{mem::transmute::<&idtree::Arena<T>, &idtree::Arena<T>>(&self.arena)})) },
            next: 0,
            forest: self,
        }
    }
}

impl<'a, T> IntoOrderedIterMut for RwLockWriteGuard<'a, Forest<T>>{
    type OrderedIterMut = ForestHierarchicalIterMut<'a,T>;
    fn into_ordered_iter_mut(self) -> Self::OrderedIterMut{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_ordered_iter_mut()
    }
}

pub struct ForestHierarchicalIterMut<'a, T: 'a>{
    forest: WriteGuardRef<'a, Forest<T>>,
    // current: Option<usize>,
    // iter: Option<idtree::Descendants<'a, T>>,
    next: usize,
}

impl<'a, T: 'a> Iterator for ForestHierarchicalIterMut<'a, T>{
    type Item = idtree::NodeRefMut<'a,T>;
    fn next(&mut self) -> Option<idtree::NodeRefMut<'a,T>>{
        if self.next == unsafe{(*self.forest.ordered_ids.get()).len()}{
            None
        }else{
            let next = unsafe{ *(*self.forest.ordered_ids.get()).get_unchecked(self.next) };
            let node = unsafe{ self.forest.get_node_mut(next) };
            self.next += 1;
            let node = unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>(node) };
            Some(node)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>){
        (self.forest.arena.len(), Some(self.forest.arena.len()))
    }
}
