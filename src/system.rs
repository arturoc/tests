pub trait System<'a>: Send{
    fn run(&mut self, ::Entities<'a>, ::Resources<'a>);
}

pub trait SystemThreadLocal<'a>{
    fn run(&mut self, ::EntitiesThreadLocal<'a>, ::ResourcesThreadLocal<'a>);
}

impl<'a, F: FnMut(::Entities<'a>, ::Resources<'a>) + Send> System<'a> for F{
    fn run(&mut self, e: ::Entities<'a>, r: ::Resources<'a>){
        (*self)(e,r)
    }
}

impl<'a, F: FnMut(::EntitiesThreadLocal<'a>, ::ResourcesThreadLocal<'a>)> SystemThreadLocal<'a> for F{
    fn run(&mut self, e: ::EntitiesThreadLocal<'a>, r: ::ResourcesThreadLocal<'a>){
        (*self)(e,r)
    }
}

pub trait SystemResources<'a>{
    fn run(&mut self, ::Resources<'a>);
}

impl<'a, F: FnMut(::Resources<'a>)> SystemResources<'a> for F{
    fn run(&mut self, e: ::Resources<'a>){
        (*self)(e)
    }
}

pub trait SystemEntities<'a>{
    fn run(&mut self, ::Entities<'a>);
}

impl<'a, F: FnMut(::Entities<'a>)> SystemEntities<'a> for F{
    fn run(&mut self, e: ::Entities<'a>){
        (*self)(e)
    }
}

pub trait SystemWithData<'a, D: Send + 'static>: Send{
    fn run(&mut self, data: &mut D, entities: ::Entities, resources: ::Resources);
}

impl<'a, D: Send + 'static, F: FnMut(&mut D,::Entities, ::Resources) + Send> SystemWithData<'a, D> for F {
    fn run(&mut self, data: &mut D, entities: ::Entities, resources: ::Resources){
        self(data, entities, resources)
    }
}

impl<'a, D: Send + 'static, S: SystemWithData<'a,D> + Send> System<'a> for (S, D){
    fn run(&mut self, entities: ::Entities, resources: ::Resources){
        self.0.run(&mut self.1, entities, resources)
    }
}

pub trait SystemWithDataThreadLocal<'a, D: 'static>{
    fn run(&mut self, data: &mut D, entities: ::EntitiesThreadLocal, resources: ::ResourcesThreadLocal);
}

impl<'a, D: 'static, F: FnMut(&mut D,::EntitiesThreadLocal, ::ResourcesThreadLocal)> SystemWithDataThreadLocal<'a, D> for F {
    fn run(&mut self, data: &mut D, entities: ::EntitiesThreadLocal, resources: ::ResourcesThreadLocal){
        self(data, entities, resources)
    }
}

impl<'a, D: 'static, S: SystemWithDataThreadLocal<'a,D>> SystemThreadLocal<'a> for (S, D){
    fn run(&mut self, entities: ::EntitiesThreadLocal, resources: ::ResourcesThreadLocal){
        self.0.run(&mut self.1, entities, resources)
    }
}

pub trait CreationSystem<'a>{
    fn run(&mut self, entities: ::EntitiesCreation<'a>, resources: ::ResourcesThreadLocal<'a>);
}

pub trait CreationSystemWithData<'a, D: 'static>{
    fn run(&mut self, data: &mut D, entities: ::EntitiesCreation<'a>, resources: ::ResourcesThreadLocal<'a>);
}

impl<'a, F: FnMut(::EntitiesCreation<'a>, ::ResourcesThreadLocal<'a>)> CreationSystem<'a> for F{
    fn run(&mut self, entities: ::EntitiesCreation<'a>, resources: ::ResourcesThreadLocal<'a>){
        self(entities, resources)
    }
}


impl<'a, D: 'static, F: FnMut(&mut D, ::EntitiesCreation<'a>, ::ResourcesThreadLocal<'a>)> CreationSystemWithData<'a, D> for F{
    fn run(&mut self, data: &mut D, entities: ::EntitiesCreation<'a>, resources: ::ResourcesThreadLocal<'a>){
        self(data, entities, resources)
    }
}

impl<'a, D: 'static, S: CreationSystemWithData<'a, D>> CreationSystem<'a> for (S, D) {
    fn run(&mut self, entities: ::EntitiesCreation<'a>, resources: ::ResourcesThreadLocal<'a>){
        self.0.run(&mut self.1, entities, resources)
    }
}