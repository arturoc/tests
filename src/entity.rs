use ::World;
use ::Component;
use ::ComponentSync;
use ::ComponentThreadLocal;
use ::UnorderedData;
use ::UnorderedDataLocal;
use ::OrderedData;
use ::OrderedDataLocal;
use ::Storage;
use ::OneToNStorage;
use ::HierarchicalStorage;
use component::OneToNComponentSync;
use sync::{ReadGuardRef, ReadGuard, Ptr};

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

pub struct EntityBuilder<'a, 'b> where 'a: 'b{
    world: &'b mut World<'a>,
    guid: usize,
    components_mask: usize,
}

impl<'a, 'b> EntityBuilder<'a, 'b>{
    pub fn new(world: &'b mut World<'a>) -> EntityBuilder<'a, 'b>{
        let next_guid = world.next_guid();
        EntityBuilder{
            guid: next_guid,
            world: world,
            components_mask: 0,
        }
    }

    pub fn build(&mut self) -> Entity{
        let entity = Entity{
            guid: self.guid,
            components_mask: self.components_mask,
        };
        self.world.push_entity(entity.clone());
        entity
    }

    pub fn add<C: ComponentSync<'a> + 'a>(&mut self, component: C) -> &mut Self {
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&C::type_id()];
        self
    }

    pub fn add_thread_local<C: ComponentThreadLocal<'a>>(&mut self, component: C) -> &mut Self {
        {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&C::type_id()];
        self
    }

    pub fn add_child<C: ComponentSync<'a>>(&mut self, parent: Entity, component: C) -> &mut Self
        where <C as Component<'a>>::Storage: HierarchicalStorage<'a,C>{
    {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                unsafe{ storage.insert_child(parent.guid, self.guid, component) }
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&C::type_id()];
        self
    }

    pub fn add_slice<C: OneToNComponentSync<'a> + Clone>(&mut self, component: &[C]) -> &mut Self{
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&C::type_id()];
        self
    }

    pub fn add_slice_thread_local<C: OneToNComponentSync<'a> + Clone>(&mut self, component: &[C]) -> &mut Self{
        {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert_slice(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&C::type_id()];
        self
    }
}


pub struct Entities<'a, 'b> where 'a: 'b{
    world: &'b ::World<'a>,
}


unsafe impl<'a, 'b> Send for Entities<'a, 'b>{}
unsafe impl<'a, 'b> Sync for Entities<'a, 'b>{}

impl<'a, 'b> Entities<'a, 'b>{
    pub(crate) fn new(world: &'b World<'a>) -> Entities<'a, 'b>{
        Entities{ world }
    }

    pub fn iter_for<S: UnorderedData<'a, 'b> + 'b>(&self) -> <S as UnorderedData<'a, 'b>>::Iter{
        S::into_iter(self.world)
    }

    pub fn ordered_iter_for<S: OrderedData<'a, 'b> + 'b>(&self) -> <S as OrderedData<'a,'b>>::Iter{
        S::into_iter(self.world)
    }

    pub fn component_for<C: ::ComponentSync<'a>>(&self, entity: Entity) -> Ptr<'a,C> {
        let storage = self.world.storage::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        Ptr::new(ReadGuardRef::new(ReadGuard::Sync(storage)), entity)
    }
}


pub struct EntitiesThreadLocal<'a, 'b> where 'a: 'b{
    world: &'b ::World<'a>,
}

impl<'a, 'b> EntitiesThreadLocal<'a, 'b>{
    pub(crate) fn new(world: &'b World<'a>) -> EntitiesThreadLocal<'a,'b>{
        EntitiesThreadLocal{ world }
    }

    pub fn iter_for<S: UnorderedDataLocal<'a,'b> + 'b>(&self) -> <S as UnorderedDataLocal<'a,'b>>::Iter{
        S::into_iter(self.world)
    }

    pub fn ordered_iter_for<S: OrderedDataLocal<'a,'b> + 'b>(&self) -> <S as OrderedDataLocal<'a,'b>>::Iter{
        S::into_iter(self.world)
    }

    pub fn component_for<C: ::ComponentSync<'a>>(&self, entity: Entity) -> Ptr<'a,C> {
        let storage = self.world.storage_thread_local::<C>()
            .expect(&format!("Trying to use non registered type {}", C::type_name()));
        Ptr::new(storage, entity)
    }
}
