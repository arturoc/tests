// based on https://github.com/SimonSapin/rust-forest/blob/master/idtree/lib.rs

#![cfg_attr(test_threads, feature(scoped))]
#![allow(dead_code)]

use std::mem;
use std::ops::{Index, IndexMut};
use std::ops::{Deref, DerefMut};
// use std::iter::Filter;
use std::slice;
use ::DenseVec;
use ::Storage;

impl<T> Deref for Node<T>{
    type Target = T;
    fn deref(&self) -> &T{
        &self.data
    }
}

impl<T> DerefMut for Node<T>{
    fn deref_mut(&mut self) -> &mut T{
        &mut self.data
    }
}


/// A node identifier within a particular `Arena`.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct NodeId {
    index: usize,  // FIXME: use NonZero to optimize the size of Option<NodeId>
    // generation: usize,
}

#[derive(Clone)]
pub struct Node<T> {
    // Keep these private (with read-only accessors) so that we can keep them consistent.
    // E.g. the parent of a node’s child is that node.
    parent: Option<NodeId>,
    previous_sibling: Option<NodeId>,
    next_sibling: Option<NodeId>,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    // alive: bool,

    id: NodeId,
    pub data: T,
}

impl<T> Node<T>{
    pub fn id(&self) -> NodeId{
        self.id
    }
}

// #[inline]
// fn is_alive<T>(node: &&Node<T>) -> bool{
//     node.alive
// }
//
// #[inline]
// fn is_alive_mut<T>(node: &&mut Node<T>) -> bool{
//     node.alive
// }

impl<T> From<Node<T>> for NodeId{
    fn from(node: Node<T>) -> NodeId{
        node.id
    }
}

#[derive(Clone)]
pub struct Arena<T> {
    nodes: DenseVec<Node<T>>,
    next_id: usize,
}

impl<T> Arena<T> {
    pub fn new() -> Arena<T> {
        Arena {
            nodes: DenseVec::new(),
            next_id: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Arena<T> {
        Arena {
            nodes: DenseVec::with_capacity(capacity),
            next_id: 0,
        }
    }

    /// Create a new node from its associated data.
    pub fn new_node(&mut self, data: T) -> NodeIdMut<T>{
        // let next_free = self.free.pop_front();
        // let id;
        // if let Some(idx) = next_free{
        //     id = NodeId {
        //         index: idx,
        //         generation: self.nodes.get(idx).id.generation + 1
        //     };
        //     mem::forget(mem::replace(&mut self.nodes[idx], Node {
        //         parent: None,
        //         first_child: None,
        //         last_child: None,
        //         previous_sibling: None,
        //         next_sibling: None,
        //         alive: true,
        //         data: data,
        //         id: id,
        //     }));
        // }else{
        //     let next_index = self.nodes.len();
        //     id = NodeId {
        //         index: next_index,
        //         generation: 0
        //     };
        //     self.nodes.push(Node {
        //         parent: None,
        //         first_child: None,
        //         last_child: None,
        //         previous_sibling: None,
        //         next_sibling: None,
        //         alive: true,
        //         data: data,
        //         id: id,
        //     });
        // }

        let id = self.next_id;
        let node_id = NodeId {
            index: self.next_id,
        };
        self.next_id += 1;
        self.nodes.insert(id, Node {
            parent: None,
            first_child: None,
            last_child: None,
            previous_sibling: None,
            next_sibling: None,
            data: data,
            id: node_id,
        });

        NodeIdMut{
            id: node_id,
            arena: self
        }
    }

    pub fn get(&self, id: NodeId) -> NodeIdRef<T>{
        NodeIdRef{
            arena: self,
            id,
        }
    }

    pub fn get_mut(&mut self, id: NodeId) -> NodeIdMut<T>{
        NodeIdMut{
            arena: self,
            id,
        }
    }

    // pub fn contains(&self, id: NodeId) -> bool{
    //     self.nodes.get(id.index).id.generation == id.generation &&
    //     self.nodes[id.index].alive
    // }

    pub fn remove<N: Into<NodeId>>(&mut self, id: N){
        let id = id.into();
        if unsafe{ self.nodes.get(id.index).parent().is_some() }{
            for c in id.children(self).collect::<Vec<_>>(){
                id.insert_after(c, self);
            }
        }else{
            for c in id.children(self).collect::<Vec<_>>(){
                c.detach(self);
            }
        }
        id.detach(self);
        self.nodes.remove(id.index);
    }


    // pub fn remove<N: Into<NodeId>>(&mut self, id: N) -> Result<(),()>{
    //     let id = id.into();
    //     if self.contains(id){
    //         if self.nodes[id.index].parent().is_some(){
    //             for c in id.children(self).collect::<Vec<_>>(){
    //                 id.insert_after(c, self);
    //             }
    //         }else{
    //             for c in id.children(self).collect::<Vec<_>>(){
    //                 c.detach(self);
    //             }
    //         }
    //         id.detach(self);
    //         unsafe{ ptr::drop_in_place(&mut self.nodes[id.index].data) };
    //         self.nodes[id.index].alive = false;
    //         self.free.push_back(id.index);
    //         Ok(())
    //     }else{
    //         Err(())
    //     }
    // }
    //
    // pub fn remove_tree<N: Into<NodeId>>(&mut self, id: N) -> Result<(),()>{
    //     let id = id.into();
    //     if self.contains(id){
    //         if self.nodes[id.index].first_child().is_some(){
    //             for c in id.children(self).collect::<Vec<_>>(){
    //                 self.remove_tree(c)?;
    //             }
    //         }else{
    //             self.remove(id)?;
    //         }
    //         Ok(())
    //     }else{
    //         Err(())
    //     }
    // }

    pub fn all_nodes(&self) -> AllNodes<T>{
        AllNodes{
            it: self.nodes.iter()
        }
    }

    pub fn all_nodes_mut(&mut self) -> AllNodesMut<T>{
        AllNodesMut{
            it: self.nodes.iter_mut()
        }
    }

    pub fn into_vec(self) -> Vec<Node<T>>{
        self.nodes.into_iter()/*.filter(|n| n.alive)*/.collect()
    }

    pub fn len(&self) -> usize{
        self.nodes.len()
    }
}

pub struct AllNodes<'a, T: 'a>{
    //t: Filter<slice::Iter<'a, Node<T>>, fn (&&Node<T>) -> bool>
    it: slice::Iter<'a, Node<T>>
}

impl<'a, T: 'a> Iterator for AllNodes<'a, T>{
    type Item = &'a Node<T>;
    #[inline]
    fn next(&mut self) -> Option<&'a Node<T>>{
        self.it.next()
    }
}


pub struct AllNodesMut<'a, T: 'a>{
    //it: Filter<slice::IterMut<'a, Node<T>>, fn (&&mut Node<T>) -> bool>
    it: slice::IterMut<'a, Node<T>>
}

impl<'a, T: 'a> Iterator for AllNodesMut<'a, T>{
    type Item = &'a mut Node<T>;
    #[inline]
    fn next(&mut self) -> Option<&'a mut Node<T>>{
        self.it.next()
    }
}

pub struct NodeIdMut<'a, T: 'a>{
    id: NodeId,
    pub(crate) arena: &'a mut Arena<T>
}

impl<'a, T: 'a> NodeIdMut<'a, T>{
    pub fn id(&self) -> NodeId{
        self.id
    }

    pub fn append<N: Into<NodeId>>(self, new_node: N) -> NodeIdMut<'a, T>{
        let new_node = new_node.into();
        self.id.append(new_node, self.arena)
    }

    pub fn append_new(self, new_data: T) -> NodeIdMut<'a, T>{
        self.id.append_new(new_data, self.arena)
    }

    pub fn prepend<N: Into<NodeId>>(self, new_node: N) -> NodeIdMut<'a, T>{
        let new_node = new_node.into();
        self.id.prepend(new_node, self.arena)
    }

    pub fn prepend_new(self, new_data: T) -> NodeIdMut<'a, T>{
        self.id.append_new(new_data, self.arena)
    }

    pub fn insert_after<N: Into<NodeId>>(self, new_node: N) -> NodeIdMut<'a, T>{
        let new_node = new_node.into();
        self.id.insert_after(new_node, self.arena)
    }

    pub fn insert_after_new(self, new_data: T) -> NodeIdMut<'a, T>{
        self.id.insert_after_new(new_data, self.arena)
    }

    pub fn insert_before<N: Into<NodeId>>(self, new_node: N) -> NodeIdMut<'a, T>{
        let new_node = new_node.into();
        self.id.insert_before(new_node, self.arena)
    }

    pub fn insert_before_new(self, new_data: T) -> NodeIdMut<'a, T>{
        self.id.insert_before_new(new_data, self.arena)
    }

    /// Return an iterator of references to this node and its ancestors.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn ancestors(self) -> Ancestors<'a,T> {
        Ancestors {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings before it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn preceding_siblings(self) -> PrecedingSiblings<'a,T> {
        PrecedingSiblings {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings after it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn following_siblings(self) -> FollowingSiblings<'a,T> {
        FollowingSiblings {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node’s children.
    pub fn children(self) -> Children<'a,T> {
        Children {
            arena: self.arena,
            node: self.arena[self.id].first_child,
        }
    }

    /// Return an iterator of references to this node’s children, in reverse order.
    pub fn reverse_children(self) -> ReverseChildren<'a,T> {
        ReverseChildren {
            arena: self.arena,
            node: self.arena[self.id].last_child,
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants(self) -> Descendants<'a,T> {
        Descendants(self.traverse())
    }


    /// Return an iterator of references to this node and its ancestors.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn ancestors_ref(self) -> AncestorsRef<'a, T> {
        AncestorsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings before it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn preceding_siblings_ref(self) -> PrecedingSiblingsRef<'a, T> {
        PrecedingSiblingsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings after it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn following_siblings_ref(self) -> FollowingSiblingsRef<'a, T> {
        FollowingSiblingsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node’s children.
    pub fn children_ref(self) -> ChildrenRef<'a, T> {
        ChildrenRef {
            arena: self.arena,
            node: self.arena[self.id].first_child,
        }
    }

    /// Return an iterator of references to this node’s children, in reverse order.
    pub fn reverse_children_ref(self) -> ReverseChildrenRef<'a, T> {
        ReverseChildrenRef {
            arena: self.arena,
            node: self.arena[self.id].last_child,
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants_ref(self) -> DescendantsRef<'a, T> {
        DescendantsRef(self.traverse())
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants_mut(self) -> DescendantsMut<'a, T> {
        DescendantsMut(
            TraverseMut {
                arena: self.arena,
                root: self.id,
                next: Some(NodeEdge::Start(self.id)),
            })
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn traverse(self) -> Traverse<'a,T> {
        Traverse {
            arena: self.arena,
            root: self.id,
            next: Some(NodeEdge::Start(self.id)),
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn reverse_traverse(self) -> ReverseTraverse<'a,T> {
        ReverseTraverse {
            arena: self.arena,
            root: self.id,
            next: Some(NodeEdge::End(self.id)),
        }
    }

    /// Return the ID of the parent node, unless this node is the root of the tree.
    pub fn parent(&self) -> Option<&T> {
        let id = self.arena[self.id].parent;
        id.map(move |id|{
            &self.arena[id].data
        })
    }

    // /// Return the ID of the first child of this node, unless it has no child.
    // pub fn first_child(&self) -> Option<NodeIdMut> { self.first_child }
    //
    // /// Return the ID of the last child of this node, unless it has no child.
    // pub fn last_child(&self) -> Option<NodeIdMut> { self.last_child }
    //
    // /// Return the ID of the previous sibling of this node, unless it is a first child.
    // pub fn previous_sibling(&self) -> Option<NodeIdMut> { self.previous_sibling }
    //
    // /// Return the ID of the previous sibling of this node, unless it is a first child.
    // pub fn next_sibling(&self) -> Option<NodeIdMut> { self.next_sibling }
}

impl<'a,T> From<NodeIdMut<'a,T>> for NodeId{
    fn from(node: NodeIdMut<'a,T>) -> NodeId{
        node.id()
    }
}

impl<'a, T: 'a> Deref for NodeIdMut<'a,T>{
    type Target = Node<T>;
    fn deref(&self) -> &Node<T>{
        &self.arena[self.id]
    }
}

impl<'a, T: 'a> DerefMut for NodeIdMut<'a,T>{
    fn deref_mut(&mut self) -> &mut Node<T>{
        &mut self.arena[self.id]
    }
}


pub struct NodeIdRef<'a, T: 'a>{
    id: NodeId,
    arena: &'a Arena<T>
}

impl<'a, T: 'a> NodeIdRef<'a, T>{
    pub fn id(&self) -> NodeId{
        self.id
    }

    /// Return an iterator of references to this node and its ancestors.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn ancestors(&self) -> Ancestors<T> {
        Ancestors {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings before it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn preceding_siblings(&self) -> PrecedingSiblings<T> {
        PrecedingSiblings {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings after it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn following_siblings(&self) -> FollowingSiblings<T> {
        FollowingSiblings {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node’s children.
    pub fn children(&self) -> Children<T> {
        Children {
            arena: self.arena,
            node: self.arena[self.id].first_child,
        }
    }

    /// Return an iterator of references to this node’s children, in reverse order.
    pub fn reverse_children(&self) -> ReverseChildren<T> {
        ReverseChildren {
            arena: self.arena,
            node: self.arena[self.id].last_child,
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants(&self) -> Descendants<T> {
        Descendants(self.traverse())
    }


    /// Return an iterator of references to this node and its ancestors.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn ancestors_ref(&self) -> AncestorsRef<T> {
        AncestorsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings before it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn preceding_siblings_ref(&self) -> PrecedingSiblingsRef<T> {
        PrecedingSiblingsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node and the siblings after it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn following_siblings_ref(&self) -> FollowingSiblingsRef<T> {
        FollowingSiblingsRef {
            arena: self.arena,
            node: Some(self.id),
        }
    }

    /// Return an iterator of references to this node’s children.
    pub fn children_ref(&self) -> ChildrenRef<T> {
        ChildrenRef {
            arena: self.arena,
            node: self.arena[self.id].first_child,
        }
    }

    /// Return an iterator of references to this node’s children, in reverse order.
    pub fn reverse_children_ref(&self) -> ReverseChildrenRef<T> {
        ReverseChildrenRef {
            arena: self.arena,
            node: self.arena[self.id].last_child,
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants_ref(&self) -> DescendantsRef<T> {
        DescendantsRef(self.traverse())
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn traverse(&self) -> Traverse<T> {
        Traverse {
            arena: self.arena,
            root: self.id,
            next: Some(NodeEdge::Start(self.id)),
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn reverse_traverse(&self) -> ReverseTraverse<T> {
        ReverseTraverse {
            arena: self.arena,
            root: self.id,
            next: Some(NodeEdge::End(self.id)),
        }
    }

    /// Return the ID of the parent node, unless this node is the root of the tree.
    pub fn parent(&self) -> Option<NodeIdRef<'a,T>> {
        let id = self.arena[self.id].parent;
        id.map(move |id|{
            NodeIdRef{
                id,
                arena: self.arena
            }
        })
    }
}

impl<'a,T> From<NodeIdRef<'a,T>> for NodeId{
    fn from(node: NodeIdRef<'a,T>) -> NodeId{
        node.id
    }
}

impl<'a, T: 'a> Deref for NodeIdRef<'a,T>{
    type Target = Node<T>;
    fn deref(&self) -> &Node<T>{
        &self.arena[self.id]
    }
}


trait GetPairMut<T> {
    /// Get mutable references to two distinct nodes
    ///
    /// Panic
    /// -----
    ///
    /// Panics if the two given IDs are the same.
    fn get_pair_mut(&mut self, a: usize, b: usize, same_index_error_message: &'static str)
                    -> (&mut T, &mut T);
}

impl<T> GetPairMut<T> for Vec<T> {
    fn get_pair_mut(&mut self, a: usize, b: usize, same_index_error_message: &'static str)
                    -> (&mut T, &mut T) {
        if a == b {
            panic!(same_index_error_message)
        }
        unsafe {
            let self2 = mem::transmute_copy::<&mut Vec<T>, &mut Vec<T>>(&self);
            (&mut self[a], &mut self2[b])
        }
    }
}

impl<T> GetPairMut<T> for DenseVec<T> {
    fn get_pair_mut(&mut self, a: usize, b: usize, same_index_error_message: &'static str)
                    -> (&mut T, &mut T) {
        if a == b {
            panic!(same_index_error_message)
        }
        unsafe {
            let self2 = mem::transmute_copy::<&mut DenseVec<T>, &mut DenseVec<T>>(&self);
            (self.get_mut(a), self2.get_mut(b))
        }
    }
}

impl<T> Index<NodeId> for Arena<T> {
    type Output = Node<T>;

    fn index(&self, node: NodeId) -> &Node<T> {
        // assert!(self.contains(node));
        unsafe{self.nodes.get(node.index)}
    }
}

impl<T> IndexMut<NodeId> for Arena<T> {
    fn index_mut(&mut self, node: NodeId) -> &mut Node<T> {
        // assert!(self.contains(node));
        unsafe{self.nodes.get_mut(node.index)}
    }
}


impl<T> Node<T> {
    /// Return the ID of the parent node, unless this node is the root of the tree.
    pub fn parent(&self) -> Option<NodeId> { self.parent }

    /// Return the ID of the first child of this node, unless it has no child.
    pub fn first_child(&self) -> Option<NodeId> { self.first_child }

    /// Return the ID of the last child of this node, unless it has no child.
    pub fn last_child(&self) -> Option<NodeId> { self.last_child }

    /// Return the ID of the previous sibling of this node, unless it is a first child.
    pub fn previous_sibling(&self) -> Option<NodeId> { self.previous_sibling }

    /// Return the ID of the previous sibling of this node, unless it is a first child.
    pub fn next_sibling(&self) -> Option<NodeId> { self.next_sibling }
}

impl NodeId {
    /// Return an iterator of references to this node and its ancestors.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn ancestors<T>(self, arena: &Arena<T>) -> Ancestors<T> {
        Ancestors {
            arena: arena,
            node: Some(self),
        }
    }

    /// Return an iterator of references to this node and the siblings before it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn preceding_siblings<T>(self, arena: &Arena<T>) -> PrecedingSiblings<T> {
        PrecedingSiblings {
            arena: arena,
            node: Some(self),
        }
    }

    /// Return an iterator of references to this node and the siblings after it.
    ///
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn following_siblings<T>(self, arena: &Arena<T>) -> FollowingSiblings<T> {
        FollowingSiblings {
            arena: arena,
            node: Some(self),
        }
    }

    /// Return an iterator of references to this node’s children.
    pub fn children<T>(self, arena: &Arena<T>) -> Children<T> {
        Children {
            arena: arena,
            node: arena[self].first_child,
        }
    }

    /// Return an iterator of references to this node’s children, in reverse order.
    pub fn reverse_children<T>(self, arena: &Arena<T>) -> ReverseChildren<T> {
        ReverseChildren {
            arena: arena,
            node: arena[self].last_child,
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    ///
    /// Parent nodes appear before the descendants.
    /// Call `.next().unwrap()` once on the iterator to skip the node itself.
    pub fn descendants<T>(self, arena: &Arena<T>) -> Descendants<T> {
        Descendants(self.traverse(arena))
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn traverse<T>(self, arena: &Arena<T>) -> Traverse<T> {
        Traverse {
            arena: arena,
            root: self,
            next: Some(NodeEdge::Start(self)),
        }
    }

    /// Return an iterator of references to this node and its descendants, in tree order.
    pub fn reverse_traverse<T>(self, arena: &Arena<T>) -> ReverseTraverse<T> {
        ReverseTraverse {
            arena: arena,
            root: self,
            next: Some(NodeEdge::End(self)),
        }
    }

    /// Detach a node from its parent and siblings. Children are not affected.
    pub fn detach<T>(self, arena: &mut Arena<T>) -> NodeIdMut<T> {
        let (parent, previous_sibling, next_sibling) = {
            let node = &mut arena[self];
            (node.parent.take(), node.previous_sibling.take(), node.next_sibling.take())
        };

        if let Some(next_sibling) = next_sibling {
            arena[next_sibling].previous_sibling = previous_sibling;
        } else if let Some(parent) = parent {
            arena[parent].last_child = previous_sibling;
        }

        if let Some(previous_sibling) = previous_sibling {
            arena[previous_sibling].next_sibling = next_sibling;
        } else if let Some(parent) = parent {
            arena[parent].first_child = next_sibling;
        }
        NodeIdMut{
            id: self,
            arena
        }
    }

    /// Append a new child to this node, after existing children.
    pub fn append<T>(self, new_child: NodeId, arena: &mut Arena<T>) -> NodeIdMut<T> {
        new_child.detach(arena);
        let last_child_opt;
        {
            let (self_borrow, new_child_borrow) = arena.nodes.get_pair_mut(
                self.index, new_child.index, "Can not append a node to itself");
            new_child_borrow.parent = Some(self);
            last_child_opt = mem::replace(&mut self_borrow.last_child, Some(new_child));
            if let Some(last_child) = last_child_opt {
                new_child_borrow.previous_sibling = Some(last_child);
            } else {
                debug_assert!(self_borrow.first_child.is_none());
                self_borrow.first_child = Some(new_child);
            }
        }
        if let Some(last_child) = last_child_opt {
            debug_assert!(arena[last_child].next_sibling.is_none());
            arena[last_child].next_sibling = Some(new_child);
        }
        NodeIdMut{
            id: new_child,
            arena
        }
    }

    pub fn append_new<T>(self, new_data: T, arena: &mut Arena<T>) -> NodeIdMut<T> {
        let new_node = arena.new_node(new_data).id();
        self.append(new_node, arena)
    }

    /// Prepend a new child to this node, before existing children.
    pub fn prepend<T>(self, new_child: NodeId, arena: &mut Arena<T>) -> NodeIdMut<T>  {
        new_child.detach(arena);
        let first_child_opt;
        {
            let (self_borrow, new_child_borrow) = arena.nodes.get_pair_mut(
                self.index, new_child.index, "Can not prepend a node to itself");
            new_child_borrow.parent = Some(self);
            first_child_opt = mem::replace(&mut self_borrow.first_child, Some(new_child));
            if let Some(first_child) = first_child_opt {
                new_child_borrow.next_sibling = Some(first_child);
            } else {
                debug_assert!(&self_borrow.first_child.is_none());
                self_borrow.last_child = Some(new_child);
            }
        }
        if let Some(first_child) = first_child_opt {
            debug_assert!(arena[first_child].previous_sibling.is_none());
            arena[first_child].previous_sibling = Some(new_child);
        }
        NodeIdMut{
            id: new_child,
            arena
        }
    }

    pub fn prepend_new<T>(self, new_data: T, arena: &mut Arena<T>) -> NodeIdMut<T> {
        let new_node = arena.new_node(new_data).id();
        self.prepend(new_node, arena)
    }

    /// Insert a new sibling after this node.
    pub fn insert_after<T>(self, new_sibling: NodeId, arena: &mut Arena<T>) -> NodeIdMut<T>  {
        new_sibling.detach(arena);
        let next_sibling_opt;
        let parent_opt;
        {
            let (self_borrow, new_sibling_borrow) = arena.nodes.get_pair_mut(
                self.index, new_sibling.index, "Can not insert a node after itself");
            parent_opt = self_borrow.parent;
            new_sibling_borrow.parent = parent_opt;
            new_sibling_borrow.previous_sibling = Some(self);
            next_sibling_opt = mem::replace(&mut self_borrow.next_sibling, Some(new_sibling));
            if let Some(next_sibling) = next_sibling_opt {
                new_sibling_borrow.next_sibling = Some(next_sibling);
            }
        }
        if let Some(next_sibling) = next_sibling_opt {
            debug_assert!(arena[next_sibling].previous_sibling.unwrap() == self);
            arena[next_sibling].previous_sibling = Some(new_sibling);
        } else if let Some(parent) = parent_opt {
            debug_assert!(arena[parent].last_child.unwrap() == self);
            arena[parent].last_child = Some(new_sibling);
        }
        NodeIdMut{
            id: new_sibling,
            arena
        }
    }

    pub fn insert_after_new<T>(self, new_data: T, arena: &mut Arena<T>) -> NodeIdMut<T> {
        let new_node = arena.new_node(new_data).id();
        self.insert_after(new_node, arena)
    }

    /// Insert a new sibling before this node.
    pub fn insert_before<T>(self, new_sibling: NodeId, arena: &mut Arena<T>) -> NodeIdMut<T>  {
        new_sibling.detach(arena);
        let previous_sibling_opt;
        let parent_opt;
        {
            let (self_borrow, new_sibling_borrow) = arena.nodes.get_pair_mut(
                self.index, new_sibling.index, "Can not insert a node before itself");
            parent_opt = self_borrow.parent;
            new_sibling_borrow.parent = parent_opt;
            new_sibling_borrow.next_sibling = Some(self);
            previous_sibling_opt = mem::replace(&mut self_borrow.previous_sibling, Some(new_sibling));
            if let Some(previous_sibling) = previous_sibling_opt {
                new_sibling_borrow.previous_sibling = Some(previous_sibling);
            }
        }
        if let Some(previous_sibling) = previous_sibling_opt {
            debug_assert!(arena[previous_sibling].next_sibling.unwrap() == self);
            arena[previous_sibling].next_sibling = Some(new_sibling);
        } else if let Some(parent) = parent_opt {
            debug_assert!(arena[parent].first_child.unwrap() == self);
            arena[parent].first_child = Some(new_sibling);
        }
        NodeIdMut{
            id: new_sibling,
            arena
        }
    }

    pub fn insert_before_new<T>(self, new_data: T, arena: &mut Arena<T>) -> NodeIdMut<T> {
        let new_node = arena.new_node(new_data).id();
        self.insert_before(new_node, arena)
    }
}


macro_rules! impl_node_iterator {
    ($name: ident, $next: expr) => {
        impl<'a, T> Iterator for $name<'a, T> {
            type Item = NodeId;

            fn next(&mut self) -> Option<NodeId> {
                match self.node.take() {
                    Some(node) => {
                        self.node = $next(&self.arena[node]);
                        Some(node)
                    }
                    None => None
                }
            }
        }
    }
}

macro_rules! impl_node_ref_iterator {
    ($name: ident, $next: expr) => {
        impl<'a, T> Iterator for $name<'a, T> {
            type Item = NodeIdRef<'a, T>;

            fn next(&mut self) -> Option<NodeIdRef<'a, T>> {
                match self.node.take() {
                    Some(node) => {
                        self.node = $next(&self.arena[node]);
                        Some(self.arena.get(node))
                    }
                    None => None
                }
            }
        }
    }
}

macro_rules! impl_node_mut_iterator {
    ($name: ident, $next: expr) => {
        impl<'a, T> Iterator for $name<'a, T> {
            type Item = NodeIdMut<'a, T>;

            fn next(&mut self) -> Option<NodeIdMut<'a, T>> {
                match self.node.take() {
                    Some(node) => {
                        self.node = $next(&self.arena[node]);
                        Some(self.arena.get_mut(node))
                    }
                    None => None
                }
            }
        }
    }
}

/// An iterator of references to the ancestors a given node.
pub struct Ancestors<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_iterator!(Ancestors, |node: &Node<T>| node.parent);

/// An iterator of references to the siblings before a given node.
pub struct PrecedingSiblings<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_iterator!(PrecedingSiblings, |node: &Node<T>| node.previous_sibling);

/// An iterator of references to the siblings after a given node.
pub struct FollowingSiblings<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_iterator!(FollowingSiblings, |node: &Node<T>| node.next_sibling);

/// An iterator of references to the children of a given node.
pub struct Children<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_iterator!(Children, |node: &Node<T>| node.next_sibling);

/// An iterator of references to the children of a given node, in reverse order.
pub struct ReverseChildren<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_iterator!(ReverseChildren, |node: &Node<T>| node.previous_sibling);


/// An iterator of references to a given node and its descendants, in tree order.
pub struct Descendants<'a, T: 'a>(Traverse<'a, T>);

impl<'a, T> Iterator for Descendants<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        loop {
            match self.0.next() {
                Some(NodeEdge::Start(node)) => return Some(node),
                Some(NodeEdge::End(_)) => {}
                None => return None
            }
        }
    }
}

/// An iterator of references to the ancestors a given node.
pub struct AncestorsRef<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_ref_iterator!(AncestorsRef, |node: &Node<T>| node.parent);

/// An iterator of references to the siblings before a given node.
pub struct PrecedingSiblingsRef<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_ref_iterator!(PrecedingSiblingsRef, |node: &Node<T>| node.previous_sibling);

/// An iterator of references to the siblings after a given node.
pub struct FollowingSiblingsRef<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_ref_iterator!(FollowingSiblingsRef, |node: &Node<T>| node.next_sibling);

/// An iterator of references to the children of a given node.
pub struct ChildrenRef<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_ref_iterator!(ChildrenRef, |node: &Node<T>| node.next_sibling);

/// An iterator of references to the children of a given node, in reverse order.
pub struct ReverseChildrenRef<'a, T: 'a> {
    arena: &'a Arena<T>,
    node: Option<NodeId>,
}
impl_node_ref_iterator!(ReverseChildrenRef, |node: &Node<T>| node.previous_sibling);

/// An iterator of references to a given node and its descendants, in tree order.
pub struct DescendantsRef<'a, T: 'a>(Traverse<'a, T>);

impl<'a, T> Iterator for DescendantsRef<'a, T> {
    type Item = NodeIdRef<'a,T>;

    fn next(&mut self) -> Option<NodeIdRef<'a,T>> {
        loop {
            match self.0.next() {
                Some(NodeEdge::Start(node)) => return Some(self.0.arena.get(node)),
                Some(NodeEdge::End(_)) => {}
                None => return None
            }
        }
    }
}
//
//
// /// An iterator of references to the ancestors a given node.
// pub struct AncestorsMut<'a, T: 'a> {
//     arena: &'a mut Arena<T>,
//     node: Option<NodeId>,
// }
// impl_node_mut_iterator!(AncestorsMut, |node: &Node<T>| node.parent);
//
// /// An iterator of references to the siblings before a given node.
// pub struct PrecedingSiblingsMut<'a, T: 'a> {
//     arena: &'a mut Arena<T>,
//     node: Option<NodeId>,
// }
// impl_node_mut_iterator!(PrecedingSiblingsMut, |node: &Node<T>| node.previous_sibling);
//
// /// An iterator of references to the siblings after a given node.
// pub struct FollowingSiblingsMut<'a, T: 'a> {
//     arena: &'a mut Arena<T>,
//     node: Option<NodeId>,
// }
// impl_node_mut_iterator!(FollowingSiblingsMut, |node: &Node<T>| node.next_sibling);
//
// /// An iterator of references to the children of a given node.
// pub struct ChildrenMut<'a, T: 'a> {
//     arena: &'a mut Arena<T>,
//     node: Option<NodeId>,
// }
// impl_node_mut_iterator!(ChildrenMut, |node: &Node<T>| node.next_sibling);
//
// /// An iterator of references to the children of a given node, in reverse order.
// pub struct ReverseChildrenMut<'a, T: 'a> {
//     arena: &'a mut Arena<T>,
//     node: Option<NodeId>,
// }
// impl_node_mut_iterator!(ReverseChildrenMut, |node: &Node<T>| node.previous_sibling);

/// An iterator of references to a given node and its descendants, in tree order.
pub struct DescendantsMut<'a, T: 'a>(TraverseMut<'a, T>);

impl<'a, T> Iterator for DescendantsMut<'a, T> {
    type Item = NodeIdMut<'a,T>;

    fn next(&mut self) -> Option<NodeIdMut<'a,T>> {
        loop {
            match self.0.next() {
                Some(NodeEdge::Start(node)) => return unsafe{mem::transmute(Some(self.0.arena.get_mut(node)))},
                Some(NodeEdge::End(_)) => {}
                None => return None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeEdge<T> {
    /// Indicates that start of a node that has children.
    /// Yielded by `Traverse::next` before the node’s descendants.
    /// In HTML or XML, this corresponds to an opening tag like `<div>`
    Start(T),

    /// Indicates that end of a node that has children.
    /// Yielded by `Traverse::next` after the node’s descendants.
    /// In HTML or XML, this corresponds to a closing tag like `</div>`
    End(T),
}
/// An iterator of references to a given node and its descendants, in tree order.
pub struct TraverseMut<'a, T: 'a> {
    arena: &'a mut Arena<T>,
    root: NodeId,
    next: Option<NodeEdge<NodeId>>,
}

impl<'a, T> Iterator for TraverseMut<'a, T> {
    type Item = NodeEdge<NodeId>;

    fn next(&mut self) -> Option<NodeEdge<NodeId>> {
        match self.next.take() {
            Some(item) => {
                self.next = match item {
                    NodeEdge::Start(node) => {
                        match self.arena[node].first_child {
                            Some(first_child) => Some(NodeEdge::Start(first_child)),
                            None => Some(NodeEdge::End(node.clone()))
                        }
                    }
                    NodeEdge::End(node) => {
                        if node == self.root {
                            None
                        } else {
                            match self.arena[node].next_sibling {
                                Some(next_sibling) => Some(NodeEdge::Start(next_sibling)),
                                None => match self.arena[node].parent {
                                    Some(parent) => Some(NodeEdge::End(parent)),

                                    // `node.parent()` here can only be `None`
                                    // if the tree has been modified during iteration,
                                    // but silently stoping iteration
                                    // seems a more sensible behavior than panicking.
                                    None => None
                                }
                            }
                        }
                    }
                };
                Some(item)
            }
            None => None
        }
    }
}

/// An iterator of references to a given node and its descendants, in tree order.
pub struct Traverse<'a, T: 'a> {
    arena: &'a Arena<T>,
    root: NodeId,
    next: Option<NodeEdge<NodeId>>,
}

impl<'a, T> Iterator for Traverse<'a, T> {
    type Item = NodeEdge<NodeId>;

    fn next(&mut self) -> Option<NodeEdge<NodeId>> {
        match self.next.take() {
            Some(item) => {
                self.next = match item {
                    NodeEdge::Start(node) => {
                        match self.arena[node].first_child {
                            Some(first_child) => Some(NodeEdge::Start(first_child)),
                            None => Some(NodeEdge::End(node.clone()))
                        }
                    }
                    NodeEdge::End(node) => {
                        if node == self.root {
                            None
                        } else {
                            match self.arena[node].next_sibling {
                                Some(next_sibling) => Some(NodeEdge::Start(next_sibling)),
                                None => match self.arena[node].parent {
                                    Some(parent) => Some(NodeEdge::End(parent)),

                                    // `node.parent()` here can only be `None`
                                    // if the tree has been modified during iteration,
                                    // but silently stoping iteration
                                    // seems a more sensible behavior than panicking.
                                    None => None
                                }
                            }
                        }
                    }
                };
                Some(item)
            }
            None => None
        }
    }
}

/// An iterator of references to a given node and its descendants, in reverse tree order.
pub struct ReverseTraverse<'a, T: 'a> {
    arena: &'a Arena<T>,
    root: NodeId,
    next: Option<NodeEdge<NodeId>>,
}

impl<'a, T> Iterator for ReverseTraverse<'a, T> {
    type Item = NodeEdge<NodeId>;

    fn next(&mut self) -> Option<NodeEdge<NodeId>> {
        match self.next.take() {
            Some(item) => {
                self.next = match item {
                    NodeEdge::End(node) => {
                        match self.arena[node].last_child {
                            Some(last_child) => Some(NodeEdge::End(last_child)),
                            None => Some(NodeEdge::Start(node.clone()))
                        }
                    }
                    NodeEdge::Start(node) => {
                        if node == self.root {
                            None
                        } else {
                            match self.arena[node].previous_sibling {
                                Some(previous_sibling) => Some(NodeEdge::End(previous_sibling)),
                                None => match self.arena[node].parent {
                                    Some(parent) => Some(NodeEdge::Start(parent)),

                                    // `node.parent()` here can only be `None`
                                    // if the tree has been modified during iteration,
                                    // but silently stoping iteration
                                    // seems a more sensible behavior than panicking.
                                    None => None
                                }
                            }
                        }
                    }
                };
                Some(item)
            }
            None => None
        }
    }
}


// #[test]
// fn it_works() {
//     use std::cell::Cell;
//
//     struct DropTracker<'a>(&'a Cell<u32>);
//     impl<'a> Drop for DropTracker<'a> {
//         fn drop(&mut self) {
//             self.0.set(&self.0.get() + 1);
//         }
//     }
//
//     let drop_counter = Cell::new(0);
//     {
//         let mut new_counter = 0;
//         let arena = &mut Arena::new();
//         macro_rules! new {
//             () => {
//                 {
//                     new_counter += 1;
//                     arena.new_node((new_counter, DropTracker(&drop_counter)))
//                 }
//             }
//         };
//
//         let a = new!();  // 1
//         a.append(new!(), arena);  // 2
//         a.append(new!(), arena);  // 3
//         a.prepend(new!(), arena);  // 4
//         let b = new!();  // 5
//         b.append(a, arena);
//         a.insert_before(new!(), arena);  // 6
//         a.insert_before(new!(), arena);  // 7
//         a.insert_after(new!(), arena);  // 8
//         a.insert_after(new!(), arena);  // 9
//         let c = new!();  // 10
//         b.append(c, arena);
//
//         assert_eq!(drop_counter.get(), 0);
//         arena[c].previous_sibling().unwrap().detach(arena);
//         assert_eq!(drop_counter.get(), 0);
//
//         assert_eq!(b.descendants(arena).map(|node| arena[node].data.0).collect::<Vec<_>>(), [
//             5, 6, 7, 1, 4, 2, 3, 9, 10
//         ]);
//     }
//
//     assert_eq!(drop_counter.get(), 10);
// }
//
//
// #[cfg(test_threads)]
// #[test]
// fn threaded() {
//     use std::thread;
//
//     let arena = &mut Arena::new();
//     let root = arena.new_node("".to_string());;
//     root.append(arena.new_node("b".to_string()), arena);
//     root.prepend(arena.new_node("a".to_string()), arena);
//     root.append(arena.new_node("c".to_string()), arena);
//
//     macro_rules! collect_data {
//         ($iter: expr) => { $iter.map(|node| &*arena[node].data).collect::<Vec<&str>>() }
//     }
//     let thread_1 = thread::scoped(|| collect_data!(root.children(arena)));
//     let thread_2 = thread::scoped(|| collect_data!(root.reverse_children(arena)));
//     assert_eq!(thread_1.join(), ["a", "b", "c"]);
//     assert_eq!(thread_2.join(), ["c", "b", "a"]);
// }
