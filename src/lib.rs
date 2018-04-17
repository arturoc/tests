#![cfg_attr(feature = "unstable", feature(test))]
#![cfg_attr(feature = "unstable", feature(get_type_id))]

extern crate rayon;
extern crate boolinator;
// extern crate fnv;
extern crate fxhash;
extern crate smallvec;
#[cfg(feature="stats_events")]
extern crate seitan;

#[cfg(feature = "bigint")]
extern crate num;

#[cfg(feature="dynamic_systems")]
extern crate libloading;
#[cfg(feature="dynamic_systems")]
extern crate notify;
#[cfg(feature="dynamic_systems")]
extern crate tempfile;

use sync::*;
use storage::*;
use bitmask::*;
pub use storage::{Read, Write, Not, ReadNot, ReadOr, ReadOption,
    Storage, IntoIter, IntoIterMut,
    ReadEntities,
    ReadHierarchical, WriteHierarchical, HierarchicalStorage,
    IntoOrderedIter, IntoOrderedIterMut, ReadAndParent, WriteAndParent,
    HierarchicalOneToNStorage,
};
pub use entity::{Entity, Entities, EntitiesThreadLocal, EntityBuilder, EntitiesCreation};
pub use component::{Component, ComponentSync, ComponentThreadLocal,
    OneToNComponent, HierarchicalOneToNComponent};
pub use dense_vec::DenseVec;
pub use forest::Forest;
pub use vec::VecStorage;
pub use resource::{Resources, ResourcesThreadLocal};
pub use world::World;
pub use system::{System, SystemThreadLocal, CreationSystem};
pub use oneton_densevec::DenseOneToNVec;
pub use assoc_vec::AssocVec;
pub use hashmap::HashMapStorage;
pub use idtree::{NodeRef, NodeRefMut, NodeId};
pub use sync::Ptr;
pub use oneton_forest::OneToNForest;
pub use creation_proxy::CreationProxy;


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
mod oneton_forest;
mod assoc_vec;
mod hashmap;
mod bitmask;
mod creation_proxy;

#[cfg(feature="dynamic_systems")]
mod dynamic_system_loader;


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
#[cfg(feature="unstable")]
mod vec_benches;
