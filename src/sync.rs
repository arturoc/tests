use std::sync::{RwLockReadGuard, RwLockWriteGuard};
use std::collections::HashMap;
use std::cell::{Ref, RefMut};
use std::mem;
use std::ops::{Deref, DerefMut};

pub struct IndexGuard<'a>{
    pub(crate) _index_guard: RwLockReadGuard<'a, HashMap<usize, Vec<usize>>>,
    pub(crate) index: &'a [usize],
}

pub enum ReadGuard<'a, S: 'a>{
    ThreadLocal(Ref<'a,S>),
    Sync(RwLockReadGuard<'a,S>),
}

pub enum WriteGuard<'a, S: 'a>{
    ThreadLocal(RefMut<'a,S>),
    Sync(RwLockWriteGuard<'a,S>),
}

impl<'a, S: 'a> Deref for ReadGuard<'a, S>{
    type Target = S;
    #[inline]
    fn deref(&self) -> &S{
        match self{
            &ReadGuard::ThreadLocal(ref s) => s.deref(),
            &ReadGuard::Sync(ref s) => s.deref(),
        }
    }
}

pub struct ReadGuardRef<'a, S: 'a>{
    _guard: ReadGuard<'a, S>,
    reference: &'a S,
}

impl<'a, S: 'a> ReadGuardRef<'a, S>{
    pub fn new(guard: ReadGuard<'a, S>) -> ReadGuardRef<'a, S>{
        ReadGuardRef{
            reference: unsafe{ mem::transmute::<&S, &S>(guard.deref()) },
            _guard: guard,
        }
    }
}

impl<'a, S: 'a> Deref for ReadGuardRef<'a, S>{
    type Target = S;
    #[inline]
    fn deref(&self) -> &S{
        self.reference
    }
}

impl<'a, S: 'a> Deref for WriteGuard<'a, S>{
    type Target = S;
    fn deref(&self) -> &S{
        match self{
            &WriteGuard::ThreadLocal(ref s) => s.deref(),
            &WriteGuard::Sync(ref s) => s.deref(),
        }
    }
}

impl<'a, S: 'a> DerefMut for WriteGuard<'a, S>{
    fn deref_mut(&mut self) -> &mut S{
        match self{
            &mut WriteGuard::ThreadLocal(ref mut s) => s.deref_mut(),
            &mut WriteGuard::Sync(ref mut s) => s.deref_mut(),
        }
    }
}

pub struct WriteGuardRef<'a, S: 'a>{
    _guard: WriteGuard<'a, S>,
    reference: &'a mut S,
}

impl<'a, S: 'a> WriteGuardRef<'a, S>{
    pub fn new(mut guard: WriteGuard<'a, S>) -> WriteGuardRef<'a, S>{
        WriteGuardRef{
            reference: unsafe{ mem::transmute::<&mut S, &mut S>(guard.deref_mut()) },
            _guard: guard,
        }
    }
}

impl<'a, S: 'a> Deref for WriteGuardRef<'a, S>{
    type Target = S;
    #[inline]
    fn deref(&self) -> &S{
        self.reference
    }
}

impl<'a, S: 'a> DerefMut for WriteGuardRef<'a, S>{
    #[inline]
    fn deref_mut(&mut self) -> &mut S{
        self.reference
    }
}
