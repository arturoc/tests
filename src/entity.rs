use std::any::TypeId;
use std::mem;

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

    pub fn build(&mut self) -> Entity{
        let entity = Entity{
            guid: self.guid,
            components_mask: self.components_mask,
        };
        self.world.push_entity(entity.clone());
        entity
    }

    pub fn add<C: ComponentSync>(&mut self, component: C) -> &mut Self {
        {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", "type_name");//C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_thread_local<C: ComponentThreadLocal>(&mut self, component: C) -> &mut Self {
        {
            let storage = self.world.storage_thread_local_mut::<C>();
            if let Some(mut storage) = storage{
                storage.insert(self.guid, component)
            }else{
                panic!("Trying to add component of type {} without registering first", "type_name");//C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_child<C: ComponentSync>(&mut self, parent: Entity, component: C) -> &mut Self
        where <C as Component>::Storage: HierarchicalStorage<C>{
    {
            let storage = self.world.storage_mut::<C>();
            if let Some(mut storage) = storage{
                unsafe{ storage.insert_child(parent.guid, self.guid, component) }
            }else{
                panic!("Trying to add component of type {} without registering first", "type_name");//C::type_name())
            }
        };
        self.components_mask |= self.world.components_mask_index[&TypeId::of::<C>()];
        self
    }

    pub fn add_slice<C: OneToNComponentSync + Clone>(&mut self, component: &[C]) -> &mut Self{
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
}


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

    pub fn component_for<C: ::ComponentSync>(&self, entity: Entity) -> &'a C{
        let storage = self.world.storage::<C>()
            .expect(&format!("Trying to use non registered type {}", "type name"));//C::type_name()));
        unsafe{ mem::transmute::<&C, &C>( storage.get(entity.guid()) ) }
    }
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

    pub fn component_for<C: ::ComponentThreadLocal>(&self, entity: Entity) -> &'a C{
        let storage = self.world.storage_thread_local::<C>()
            .expect(&format!("Trying to use non registered type {}", "type name"));//C::type_name()));
        unsafe{ mem::transmute::<&C,&C>( storage.get(entity.guid()) )}
    }
}
