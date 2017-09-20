pub trait System<'a>{
    fn run(&mut self, ::Entities<'a>, ::Resources<'a>){}
}


impl<'a, F: FnMut(::Entities<'a>, ::Resources<'a>)> System<'a> for F{
    fn run(&mut self, e: ::Entities<'a>, r: ::Resources<'a>){
        (*self)(e,r)
    }
}

pub trait SystemResources<'a>{
    fn run(&mut self, ::Resources<'a>){}
}

impl<'a, F: FnMut(::Resources<'a>)> SystemResources<'a> for F{
    fn run(&mut self, e: ::Resources<'a>){
        (*self)(e)
    }
}

pub trait SystemEntities<'a>{
    fn run(&mut self, ::Entities<'a>){}
}

impl<'a, F: FnMut(::Entities<'a>)> SystemEntities<'a> for F{
    fn run(&mut self, e: ::Entities<'a>){
        (*self)(e)
    }
}
