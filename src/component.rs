use ::Storage;

pub trait Component: 'static + Sized {
    type Storage: Storage<Self>;
    fn type_name() -> &'static str;
}

pub trait ComponentSync: Component{}
impl<C: Component + Send> ComponentSync for C{}

pub trait ComponentThreadLocal: Component{}
impl<C: Component> ComponentThreadLocal for C{}


pub trait OneToNComponent: 'static + Sized + Component<Storage = ::DenseOneToNVec<Self>>{
}

pub trait OneToNComponentSync: OneToNComponent + Send{}
impl<C: OneToNComponent + Send> OneToNComponentSync for C{}

pub trait OneToNComponentThreadLocal: OneToNComponent{}
impl<C: OneToNComponent> OneToNComponentThreadLocal for C{}
