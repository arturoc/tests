use std::any::TypeId;

use ::World;
use ::UnorderedData;
use ::UnorderedDataLocal;
use ::OrderedData;
use ::OrderedDataLocal;
use ::Storage;
use ::OneToNStorage;
use ::HierarchicalStorage;
use ::HierarchicalOneToNStorage;
use component::{Component, ComponentSync, ComponentThreadLocal,
    OneToNComponentSync, OneToNComponentThreadLocal,
    HierarchicalOneToNComponent, HierarchicalOneToNComponentSync, HierarchicalOneToNComponentThreadLocal};
use sync::{ReadGuardRef, ReadGuard, WriteGuardRef, WriteGuard, Ptr, PtrMut, NodePtr, NodePtrMut};

#[derive(Clone,Copy,Eq,PartialEq,Debug)]
pub struct Entity {
    guid: usize,
    pub(crate) components_mask: usize,
}

impl Entity{
    pub fn guid(&self) -> usize{
        self.guid
    }
}

pub struct EntityBuilder<'a>{
    world: &'a mut World,
    guid: usize,
    components_mask: usize,
}

impl<'a> EntityBuilder<'a>{
    pub fn new(world: &'a mut World) -> EntityBuilder{
        let next_guid = world.next_guid();
        EntityBuilder{
            guid: next_guid,
            world: world,
            components_mask: 0,
        }
    }

    pub fn build(self) -> Entity{
        let entity = Entity{
            guid: self.guid,
            components_mask: self.components_mask,
        };
        self.world.push_entity(entity.clone());
        entity
    }

    pub fn add<C: ComponentSync + 'a>(mut self, component: C) -> Self {
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_thread_local<C: ComponentThreadLocal>(mut self, component: C) -> Self {
        {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_child<C: ComponentSync>(mut self, parent: &Entity, component: C) -> Self
        where <C as Component>::Storage: HierarchicalStorage<'a,C>{
    {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                unsafe{ storage.insert_child(parent.guid, self.guid, component) }
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_child_thread_local<C: ComponentThreadLocal>(mut self, parent: Entity, component: C) -> Self
        where <C as Component>::Storage: HierarchicalStorage<'a,C>{
    {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                unsafe{ storage.insert_child(parent.guid, self.guid, component) }
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_slice<C: OneToNComponentSync + Clone>(mut self, component: &[C]) -> Self{
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_slice_thread_local<C: OneToNComponentThreadLocal + Clone>(mut self, component: &[C]) -> Self{
        {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_hierarchy<C: HierarchicalOneToNComponentSync>(&mut self) -> HierarchyBuilder<C>{
        let storage = self.world.storage_mut::<C>();
        if let Some(storage) = storage{
            self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
            let storage = WriteGuardRef::new(WriteGuard::Sync(storage));
            HierarchyBuilder{
                entity: self.guid,
                storage
            }
        }else{
            panic!("Trying to add component of type {} without registering first", C::type_name())
        }
    }

    pub fn add_hierarchy_thread_local<C: HierarchicalOneToNComponentThreadLocal>(&mut self) -> HierarchyBuilder<C>{
        let storage = self.world.storage_thread_local_mut::<C>();
        if let Some(storage) = storage{
            self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
            HierarchyBuilder{
                entity: self.guid,
                storage
            }
        }else{
            panic!("Trying to add component of type {} without registering first", C::type_name())
        }
    }
}

pub struct HierarchyBuilder<'a, T:HierarchicalOneToNComponent>{
    entity: usize,
    storage: WriteGuardRef<'a, <T as Component>::Storage>,
}

impl<'a, T:HierarchicalOneToNComponent> HierarchyBuilder<'a, T>{
    pub fn new_node(&mut self, t: T) -> ::NodeId{
        unsafe{ self.storage.insert_root(self.entity, t).id() }
    }

    pub fn append_child(&mut self, parent: ::NodeId, t: T) -> ::NodeId {
        unsafe{ self.storage.insert_child(parent, t).id() }
    }
}

#[derive(Clone, Copy)]
pub struct Entities<'a>{
    world: &'a ::World,
}


unsafe impl<'a> Send for Entities<'a>{}
unsafe impl<'a> Sync for Entities<'a>{}

impl<'a> Entities<'a>{
    pub(crate) fn new(world: &World) -> Entities{
        Entities{ world }
    }

    pub fn iter_for<S: UnorderedData<'a> + 'a>(&self) -> <S as UnorderedData<'a>>::Iter{
        S::into_iter(self.world)
    }

    pub fn ordered_iter_for<S: OrderedData<'a> + 'a>(&self) -> <S as OrderedData<'a>>::Iter{
        S::into_iter(self.world)
    }

    pub fn component_for<C: ::ComponentSync>(&self, entity: &Entity) -> Ptr<'a,C> {
        let storage = self.world.storage::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        Ptr::new(ReadGuardRef::new(ReadGuard::Sync(storage)), *entity)
    }

    pub fn component_for_mut<C: ::ComponentSync>(&self, entity: &Entity) -> PtrMut<'a,C> {
        let storage = self.world.storage_mut::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        PtrMut::new(WriteGuardRef::new(WriteGuard::Sync(storage)), *entity)
    }

    pub fn tree_node_for<C: ::Component>(&self, entity: &Entity) -> NodePtr<'a, C>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'a, C>
    {
        let storage = self.world.storage_thread_local::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        NodePtr::new(storage, *entity)
    }

    pub fn tree_node_for_mut<C: ::Component>(&self, entity: &Entity) -> NodePtrMut<'a, C>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'a, C>
    {
        let storage = self.world.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        NodePtrMut::new(storage, *entity)
    }

    // TODO: Is this useful? as it is it's not safe as there's no guard for the storage being kept
    // for the lifetime of the reference
    // pub fn get<S: UnorderedData<'a> + 'a>(&self, entity: &Entity) -> <S as UnorderedData<'a>>::ComponentsRef
    //     where <S as UnorderedData<'a>>::Storage: StorageRef<'a, <S as UnorderedData<'a>>::ComponentsRef>
    // {
    //     S::storage(self.world).get(entity.guid())
    // }
}


pub struct EntitiesThreadLocal<'a>{
    world: &'a ::World,
}

impl<'a> EntitiesThreadLocal<'a>{
    pub(crate) fn new(world: &World) -> EntitiesThreadLocal{
        EntitiesThreadLocal{ world }
    }

    pub fn iter_for<S: UnorderedDataLocal<'a> + 'a>(&self) -> <S as UnorderedDataLocal<'a>>::Iter{
        S::into_iter(self.world)
    }

    pub fn ordered_iter_for<S: OrderedDataLocal<'a> + 'a>(&self) -> <S as OrderedDataLocal<'a>>::Iter{
        S::into_iter(self.world)
    }

    pub fn component_for<C: ::Component>(&self, entity: &Entity) -> Ptr<'a,C> {
        let storage = self.world.storage_thread_local::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        Ptr::new(storage, *entity)
    }

    pub fn component_for_mut<C: ::Component>(&self, entity: &Entity) -> PtrMut<'a,C> {
        let storage = self.world.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        PtrMut::new(storage, *entity)
    }

    pub fn tree_node_for<C: ::Component>(&self, entity: &Entity) -> NodePtr<'a, C>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'a, C>
    {
        let storage = self.world.storage_thread_local::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        NodePtr::new(storage, *entity)
    }

    pub fn tree_node_for_mut<C: ::Component>(&self, entity: &Entity) -> NodePtrMut<'a, C>
        where <C as ::Component>::Storage: ::HierarchicalStorage<'a, C>
    {
        let storage = self.world.storage_thread_local_mut::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        NodePtrMut::new(storage, *entity)
    }

    // TODO: Is this useful? as it is it's not safe as there's no guard for the storage being kept
    // for the lifetime of the reference
    // pub fn get<S: UnorderedData<'a> + 'a>(&self, entity: &Entity) -> <S as UnorderedData<'a>>::ComponentsRef
    //     where <S as UnorderedData<'a>>::Storage: StorageRef<'a, <S as UnorderedData<'a>>::ComponentsRef>
    // {
    //     S::storage(self.world).get(entity.guid())
    // }
}
