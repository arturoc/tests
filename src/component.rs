use ::Storage;

pub trait Component: 'static + Sized {
    type Storage: Storage<Self>;
}

pub trait ComponentSync: Component{}
impl<C: Component + Send> ComponentSync for C{}

pub trait ComponentThreadLocal: Component{}
impl<C: Component> ComponentThreadLocal for C{}
