use std::cell::{Ref,RefMut};

pub struct Resources<'a>{
    world: &'a ::World<'a>
}

impl<'a> Resources<'a>{
    pub fn new(world: &'a ::World<'a>) -> Resources{
        Resources{ world }
    }

    pub fn get<T: 'static>(&self) -> Option<Ref<T>>{
        self.world.resource::<T>()
    }

    pub fn get_mut<T: 'static>(&self) -> Option<RefMut<T>>{
        self.world.resource_mut::<T>()
    }
}
