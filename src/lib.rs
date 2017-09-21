#![cfg_attr(feature = "unstable", feature(test))]
#![cfg_attr(feature = "unstable", feature(get_type_id))]

#[cfg(test)]
extern crate rayon;


use sync::*;
use storage::*;
pub use storage::{Read, Write, Storage, IntoIter, IntoIterMut,
    ReadEntities,
    ReadHierarchical, WriteHierarchical, HierarchicalStorage,
    IntoOrderedIter, IntoOrderedIterMut, ReadAndParent, WriteAndParent,
};
pub use entity::{Entity, Entities, EntitiesThreadLocal, EntityBuilder};
pub use component::{Component, ComponentSync, ComponentThreadLocal, OneToNComponent};
pub use dense_vec::DenseVec;
pub use forest::Forest;
pub use vec::VecStorage;
pub use resource::Resources;
pub use world::World;
pub use system::*;
pub use oneton_densevec::DenseOneToNVec;

mod sync;
mod entity;
mod component;
mod storage;
mod dense_vec;
mod forest;
mod idtree;
mod vec;
mod resource;
mod world;
mod system;
mod oneton_densevec;

#[cfg(test)]
mod tests;

#[cfg(feature="unstable")]
mod benches;
#[cfg(feature="unstable")]
mod parallel_benches;
#[cfg(feature="unstable")]
mod hierarchical_benches;
#[cfg(feature="unstable")]
mod one_to_n_benches;
