use std::{cell::Cell, collections::HashMap, rc::Rc};

use slotmap::SparseSecondaryMap;

use crate::types::{Link, LinkKey, NodeInner, NodeKey, ReactiveFlags, ReactiveNode, UnsafeSlotMap};

mod batching;
mod computed;
mod context;
mod effect;
mod lifecycle;
mod links;
mod propagation;
mod signal;

type NodeMap = UnsafeSlotMap<NodeKey, ReactiveNode>;
type LinkMap = UnsafeSlotMap<LinkKey, Link>;

pub struct ReactiveSystem {
    pub cycle: usize,
    pub batch_depth: usize,
    pub notify_index: usize,
    pub queued_length: usize,
    pub queued: Vec<Option<NodeKey>>,
    pub active_sub: Cell<Option<NodeKey>>,
    pub root: NodeKey,
    pub current_scope: Cell<NodeKey>,
    pub nodes: NodeMap,
    pub links: LinkMap,
    pub cleanups: SparseSecondaryMap<NodeKey, Vec<Box<dyn FnOnce()>>>,
    pub contexts: SparseSecondaryMap<NodeKey, HashMap<std::any::TypeId, Rc<dyn std::any::Any>>>,
    pub check_dirty_stack: Vec<LinkKey>,
    pub propagate_stack: Vec<Option<LinkKey>>,
}

impl ReactiveSystem {
    pub fn new() -> Self {
        let mut nodes: NodeMap = Default::default();
        let links: LinkMap = Default::default();
        let cleanups = SparseSecondaryMap::new();
        let contexts = SparseSecondaryMap::new();

        // Create root scope node (no parent, so scope = None)
        let root = nodes.insert(ReactiveNode::new(
            NodeInner::None,
            ReactiveFlags::NONE,
            None,
        ));

        Self {
            cycle: 0,
            batch_depth: 0,
            notify_index: 0,
            queued_length: 0,
            active_sub: Cell::new(None),
            root,
            current_scope: Cell::new(root),
            nodes,
            links,
            cleanups,
            contexts,
            queued: Vec::new(),
            check_dirty_stack: Vec::new(),
            propagate_stack: Vec::new(),
        }
    }
}
