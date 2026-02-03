use crate::types::{
    Link, LinkKey, NodeInner, NodeKey, ReactiveFlags, ReactiveNode, UnsafeBox, UnsafeSlotMap,
    caller,
};
use serde::Serialize;
use slotmap::SparseSecondaryMap;
use std::{cell::Cell, collections::HashMap, rc::Rc};

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

#[derive(Default, Serialize)]
pub struct ReactiveSystem {
    pub cycle: usize,
    pub batch_depth: usize,
    pub notify_index: usize,
    pub queued_length: usize,
    #[serde(skip)]
    pub queued: Vec<NodeKey>,
    #[serde(skip)]
    pub stack: Vec<LinkKey>,
    pub root: NodeKey,
    #[serde(skip)]
    pub active_sub: Cell<Option<NodeKey>>,
    #[serde(skip)]
    pub current_scope: Cell<NodeKey>,
    pub nodes: NodeMap,
    pub links: LinkMap,
    #[serde(skip)]
    pub cleanups: SparseSecondaryMap<NodeKey, Vec<Box<dyn FnOnce()>>>,
    #[serde(skip)]
    pub contexts: SparseSecondaryMap<NodeKey, HashMap<std::any::TypeId, Rc<dyn std::any::Any>>>,
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
            caller(),
        ));

        Self {
            root,
            current_scope: Cell::new(root),
            nodes,
            links,
            cleanups,
            contexts,
            ..Default::default()
        }
    }
}

// #[cfg(debug_assertions)]
// pub type ReactiveSystemRef<T> = Rc<RefCell<T>>;

// #[cfg(not(debug_assertions))]
pub type ReactiveSystemRef<T> = UnsafeBox<T>;
