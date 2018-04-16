
use component::{ComponentSync, ComponentThreadLocal,
    OneToNComponentSync, OneToNComponentThreadLocal};

use entity::{Entity, EntityBuilder, EntitiesCreation};
use storage::{UnorderedDataLocal, OrderedDataLocal};
use sync::{NodePtr, Ptr, PtrMut, NodePtrMut};
use world::World;

pub trait CreationProxy {
    fn iter_for<'e, S: UnorderedDataLocal<'e> + 'e>(&'e self) -> <S as UnorderedDataLocal<'e>>::Iter;
    fn ordered_iter_for<'e, S: OrderedDataLocal<'e> + 'e>(&'e self) -> <S as OrderedDataLocal<'e>>::Iter;
    fn component_for<C: ::Component>(&self, entity: &Entity) -> Option<Ptr<C>> ;
    fn component_for_mut<C: ::Component>(&self, entity: &Entity) -> Option<PtrMut<C>> ;
    fn tree_node_for<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtr<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>;
    fn tree_node_for_mut<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtrMut<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>;
    fn create_entity(&mut self) -> EntityBuilder;
    fn add_component_to<C: ComponentSync>(&mut self, entity: &Entity, component: C);
    fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C);
    fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]);
    fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]);
    fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity);
    fn remove_entity(&mut self, entity: &::Entity);
}

impl<'a> CreationProxy for EntitiesCreation<'a>{
    fn iter_for<'e, S: UnorderedDataLocal<'e> + 'e>(&'e self) -> <S as UnorderedDataLocal<'e>>::Iter{
        self.iter_for::<S>()
    }

    fn ordered_iter_for<'e, S: OrderedDataLocal<'e> + 'e>(&'e self) -> <S as OrderedDataLocal<'e>>::Iter{
        self.ordered_iter_for::<S>()
    }

    fn component_for<C: ::Component>(&self, entity: &Entity) -> Option<Ptr<C>> {
        self.component_for::<C>(entity)
    }

    fn component_for_mut<C: ::Component>(&self, entity: &Entity) -> Option<PtrMut<C>> {
        self.component_for_mut::<C>(entity)
    }

    fn tree_node_for<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtr<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>
    {
        self.tree_node_for::<C>(entity)
    }

    fn tree_node_for_mut<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtrMut<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>
    {
        self.tree_node_for_mut::<C>(entity)
    }

    fn create_entity(&mut self) -> EntityBuilder{
        self.create_entity()
    }

    fn add_component_to<C: ComponentSync>(&mut self, entity: &Entity, component: C){
        self.add_component_to(entity, component)
    }

    fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C){
        self.add_component_to_thread_local(entity, component)
    }

    fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]){
        self.add_slice_component_to(entity, component)
    }

    fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]){
        self.add_slice_component_to_thread_local(entity, component)
    }

    fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        self.remove_component_from::<C>(entity)
    }

    fn remove_entity(&mut self, entity: &::Entity){
        self.remove_entity(entity)
    }
}



impl CreationProxy for World{
    fn iter_for<'e, S: UnorderedDataLocal<'e> + 'e>(&'e self) -> <S as UnorderedDataLocal<'e>>::Iter{
        self.entities_thread_local().iter_for::<S>()
    }

    fn ordered_iter_for<'e, S: OrderedDataLocal<'e> + 'e>(&'e self) -> <S as OrderedDataLocal<'e>>::Iter{
        self.entities_thread_local().ordered_iter_for::<S>()
    }

    fn component_for<C: ::Component>(&self, entity: &Entity) -> Option<Ptr<C>> {
        self.entities_thread_local().component_for::<C>(entity)
    }

    fn component_for_mut<C: ::Component>(&self, entity: &Entity) -> Option<PtrMut<C>> {
        self.entities_thread_local().component_for_mut::<C>(entity)
    }

    fn tree_node_for<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtr<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>
    {
        self.entities_thread_local().tree_node_for::<C>(entity)
    }

    fn tree_node_for_mut<'e, C: ::Component>(&'e self, entity: &Entity) -> Option<NodePtrMut<'e, C>>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'e, C>
    {
        self.entities_thread_local().tree_node_for_mut::<C>(entity)
    }

    fn create_entity(&mut self) -> EntityBuilder{
        self.create_entity()
    }

    fn add_component_to<C: ComponentSync>(&mut self, entity: &Entity, component: C){
        self.add_component_to(entity, component)
    }

    fn add_component_to_thread_local<C: ComponentThreadLocal>(&mut self, entity: &Entity, component: C){
        self.add_component_to_thread_local(entity, component)
    }

    fn add_slice_component_to<C: OneToNComponentSync>(&mut self, entity: &Entity, component: &[C]){
        self.add_slice_component_to(entity, component)
    }

    fn add_slice_component_to_thread_local<C: OneToNComponentThreadLocal>(&mut self, entity: &Entity, component: &[C]){
        self.add_slice_component_to_thread_local(entity, component)
    }

    fn remove_component_from<C: ::Component>(&mut self, entity: &::Entity){
        self.remove_component_from::<C>(entity)
    }

    fn remove_entity(&mut self, entity: &::Entity){
        self.remove_entity(entity)
    }
}