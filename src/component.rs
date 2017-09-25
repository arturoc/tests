use storage::Storage;
use std::any::TypeId;

// Component
pub trait Component<'a>: 'a + Sized{
    type Storage: Storage<'a,Self>;
    type Key: 'static;
    fn type_name() -> &'static str;
    fn type_id() -> TypeId{
        TypeId::of::<Self::Key>()
    }
}

pub trait ComponentSync<'a>: Component<'a>{}
impl<'a, C: Component<'a> + Send> ComponentSync<'a> for C{}

pub trait ComponentThreadLocal<'a>: Component<'a>{}
impl<'a, C: Component<'a>> ComponentThreadLocal<'a> for C{}



// OneToN
pub trait OneToNComponent<'a>: 'a + Sized + Component<'a, Storage = ::DenseOneToNVec<Self>> where Self: Clone{}

pub trait OneToNComponentSync<'a>: OneToNComponent<'a> + Send{}
impl<'a, C: OneToNComponent<'a> + Send> OneToNComponentSync<'a> for C{}

pub trait OneToNComponentThreadLocal<'a>: OneToNComponent<'a>{}
impl<'a, C: OneToNComponent<'a>> OneToNComponentThreadLocal<'a> for C{}



// Hierarchical OneToN
pub trait HierarchicalOneToNComponent<'a>: 'a + Sized + Component<'a, Storage = ::OneToNForest<Self>> where Self: Clone{}

pub trait HierarchicalOneToNComponentSync<'a>: HierarchicalOneToNComponent<'a> + Send{}
impl<'a, C: HierarchicalOneToNComponent<'a> + Send> HierarchicalOneToNComponentSync<'a> for C{}

pub trait HierarchicalOneToNComponentThreadLocal<'a>: HierarchicalOneToNComponent<'a>{}
impl<'a, C: HierarchicalOneToNComponent<'a>> HierarchicalOneToNComponentThreadLocal<'a> for C{}
