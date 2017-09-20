use ::Storage;

pub trait Component: 'static + Sized {
    type Storage: Storage<Self>;
    fn type_name() -> &'static str;
}

pub trait ComponentSync: Component{}
impl<C: Component + Send> ComponentSync for C{}

pub trait ComponentThreadLocal: Component{}
impl<C: Component> ComponentThreadLocal for C{}
