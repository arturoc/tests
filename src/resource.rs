use std::cell::{Ref,RefMut};

pub struct Resources<'a>{
    world: &'a ::World
}

unsafe impl<'a> Send for Resources<'a>{}

impl<'a> Resources<'a>{
    pub fn new(world: &::World) -> Resources{
        Resources{ world }
    }

    pub fn get<T: 'static + Send>(&self) -> Option<Ref<T>>{
        self.world.resource::<T>()
    }

    pub fn get_mut<T: 'static + Send>(&self) -> Option<RefMut<T>>{
        self.world.resource_mut::<T>()
    }
}

pub struct ResourcesThreadLocal<'a>{
    world: &'a ::World
}

impl<'a> ResourcesThreadLocal<'a>{
    pub fn new(world: &::World) -> ResourcesThreadLocal{
        ResourcesThreadLocal{ world }
    }

    pub fn get<T: 'static>(&self) -> Option<Ref<T>>{
        self.world.resource_thread_local::<T>()
    }

    pub fn get_mut<T: 'static>(&self) -> Option<RefMut<T>>{
        self.world.resource_thread_local_mut::<T>()
    }
}
