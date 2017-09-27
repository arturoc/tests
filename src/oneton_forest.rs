use idtree;
use storage::{Storage, HierarchicalOneToNStorage, IntoIter, IntoIterMut};
use ::DenseVec;
use sync::{ReadGuardRef, ReadGuard, WriteGuardRef, WriteGuard};

use std::slice;
use std::mem;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

pub struct OneToNForest<T>{
    arena: idtree::Arena<T>,
    entities_roots: DenseVec<Vec<idtree::NodeId>>,
}

impl<'a, T: 'a> Storage<'a, T> for OneToNForest<T>{
    type Target = T;
    type Get = RootsIter<'a, T>;
    type GetMut = RootsIterMut<'a, T>;

    fn new() -> OneToNForest<T>{
        OneToNForest{
            arena: idtree::Arena::new(),
            entities_roots: DenseVec::new(),
        }
    }

    fn with_capacity(capacity: usize) -> OneToNForest<T>{
        OneToNForest{
            arena: idtree::Arena::with_capacity(capacity),
            entities_roots: DenseVec::with_capacity(capacity),
        }
    }

    fn insert(&mut self, guid: usize, t: T){
        unsafe{ self.insert_root(guid, t) };
    }

    fn remove(&mut self, guid: usize){
        for id in unsafe{ self.entities_roots.get(guid).iter() }{
            self.arena.remove_tree(*id);
        }
        self.entities_roots.remove(guid);
    }

    unsafe fn get(&'a self, guid: usize) -> RootsIter<'a,T>{
        RootsIter{
            iter: self.entities_roots.get(guid).iter(),
            arena: &self.arena
        }
    }

    unsafe fn get_mut(&'a mut self, guid: usize) -> RootsIterMut<'a,T>{
        RootsIterMut{
            iter: self.entities_roots.get(guid).iter(),
            arena: &mut self.arena
        }
    }

    unsafe fn get_for_ptr(&self, guid: usize) -> &Self::Target{
        &self.arena[*self.entities_roots.get(guid).get_unchecked(0)]
    }

    unsafe fn get_for_ptr_mut(&mut self, guid: usize) -> &mut Self::Target{
        &mut self.arena[*self.entities_roots.get(guid).get_unchecked(0)]
    }
}

impl<'a, T: 'a> HierarchicalOneToNStorage<'a,T> for OneToNForest<T>{
    unsafe fn insert_root(&mut self, guid: usize, t: T) -> idtree::NodeRefMut<T>{
        let root = self.arena.new_node(t);
        self.entities_roots.entry(guid)
            .or_insert_with(|| vec![])
            .push(root.id());
        root
    }

    unsafe fn insert_child(&mut self, parent: idtree::NodeId, t: T) -> idtree::NodeRefMut<T>{
        parent.append_new(t, &mut self.arena)
    }
}


pub struct RootsIter<'a, T: 'a>{
    iter: slice::Iter<'a, idtree::NodeId>,
    arena: &'a idtree::Arena<T>,
}

impl<'a, T: 'a> Iterator for RootsIter<'a, T>{
    type Item = idtree::NodeRef<'a,T>;

    fn next(&mut self) -> Option<idtree::NodeRef<'a, T>>{
        self.iter.next().map(|id| self.arena.get(*id))
    }
}

pub struct RootsIterMut<'a, T: 'a>{
    iter: slice::Iter<'a, idtree::NodeId>,
    arena: &'a mut idtree::Arena<T>,
}

impl<'a, T: 'a> Iterator for RootsIterMut<'a, T>{
    type Item = idtree::NodeRefMut<'a,T>;

    fn next(&mut self) -> Option<idtree::NodeRefMut<'a, T>>{
        self.iter.next().map(|id| unsafe{ mem::transmute::<idtree::NodeRefMut<T>, idtree::NodeRefMut<T>>(self.arena.get_mut(*id)) } )
    }
}


pub struct Iter<'a, T: 'a>{
    storage: ReadGuardRef<'a, OneToNForest<T>>,
    iter: slice::Iter<'a, Vec<idtree::NodeId>>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T>{
    type Item = RootsIter<'a, T>;
    fn next(&mut self) -> Option<RootsIter<'a, T>>{
        self.iter.next().map(|roots| RootsIter{
            iter: roots.iter(),
            arena: unsafe{ mem::transmute::<&idtree::Arena<T>, &idtree::Arena<T>>(&self.storage.arena) },
        })
    }
}

impl<'a, T> IntoIter for ReadGuardRef<'a, OneToNForest<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        Iter{
            iter: unsafe{ mem::transmute::<slice::Iter<Vec<idtree::NodeId>>, slice::Iter<Vec<idtree::NodeId>>>(self.entities_roots.iter()) },
            storage: self,
        }
    }
}

impl<'a, T> IntoIter for RwLockReadGuard<'a, OneToNForest<T>>{
    type Iter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T>{
        ReadGuardRef::new(ReadGuard::Sync(self)).into_iter()
    }
}



pub struct IterMut<'a, T: 'a>{
    storage: WriteGuardRef<'a, OneToNForest<T>>,
    iter: slice::Iter<'a, Vec<idtree::NodeId>>,
}

impl<'a, T: 'a> Iterator for IterMut<'a, T>{
    type Item = RootsIterMut<'a, T>;
    fn next(&mut self) -> Option<RootsIterMut<'a, T>>{
        self.iter.next().map(|roots| RootsIterMut{
            iter: roots.iter(),
            arena: unsafe{ mem::transmute::<&mut idtree::Arena<T>, &mut idtree::Arena<T>>(&mut self.storage.arena) },
        })
    }
}

impl<'a, T> IntoIterMut for WriteGuardRef<'a, OneToNForest<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(self) -> IterMut<'a, T>{
        IterMut{
            iter: unsafe{ mem::transmute::<slice::Iter<Vec<idtree::NodeId>>, slice::Iter<Vec<idtree::NodeId>>>(self.entities_roots.iter()) },
            storage: self,
        }
    }
}

impl<'a, T> IntoIterMut for RwLockWriteGuard<'a, OneToNForest<T>>{
    type IterMut = IterMut<'a, T>;
    fn into_iter_mut(self) -> IterMut<'a, T>{
        WriteGuardRef::new(WriteGuard::Sync(self)).into_iter_mut()
    }
}
