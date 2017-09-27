use ::Storage;

pub trait Component: 'static + Sized {
    type Storage: for<'a> Storage<'a, Self>;
    fn type_name() -> &'static str;
}

pub trait ComponentSync: Component{}
impl<C: Component + Send> ComponentSync for C{}

pub trait ComponentThreadLocal: Component{}
impl<C: Component> ComponentThreadLocal for C{}


// OneToN
pub trait OneToNComponent: 'static + Sized + Component<Storage = ::DenseOneToNVec<Self>> where Self: Clone{
}

pub trait OneToNComponentSync: OneToNComponent + Send{}
impl<C: OneToNComponent + Send> OneToNComponentSync for C{}

pub trait OneToNComponentThreadLocal: OneToNComponent{}
impl<C: OneToNComponent> OneToNComponentThreadLocal for C{}




// Hierarchical OneToN
pub trait HierarchicalOneToNComponent: 'static + Sized + Component<Storage = ::OneToNForest<Self>>{}

pub trait HierarchicalOneToNComponentSync: HierarchicalOneToNComponent + Send{}
impl<'a, C: HierarchicalOneToNComponent + Send> HierarchicalOneToNComponentSync for C{}

pub trait HierarchicalOneToNComponentThreadLocal: HierarchicalOneToNComponent{}
impl<'a, C: HierarchicalOneToNComponent> HierarchicalOneToNComponentThreadLocal for C{}
