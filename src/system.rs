pub trait System<'a,'b>{
    fn run(&mut self, ::Entities<'a,'b>, ::Resources<'a>){}
}


impl<'a,'b, F: FnMut(::Entities<'a,'b>, ::Resources<'a>)> System<'a,'b> for F where 'a: 'b{
    fn run(&mut self, e: ::Entities<'a,'b>, r: ::Resources<'a>){
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

pub trait SystemEntities<'a,'b>{
    fn run(&mut self, ::Entities<'a,'b>){}
}

impl<'a,'b, F: FnMut(::Entities<'a,'b>)> SystemEntities<'a,'b> for F
    where 'a: 'b
{
    fn run(&mut self, e: ::Entities<'a,'b>){
        (*self)(e)
    }
}
