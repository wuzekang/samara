use crate::system::ReactiveSystemRef;
use crate::types::{
    ComputedNodeInner, ComputedOps, MemoNodeInner, NodeInner, NodeKey, ReactiveFlags, ReactiveNode,
};
use crate::types::{Location, RefCell};

impl super::ReactiveSystem {
    /// Create a new memo node (with equality check)
    pub fn computed_memo<F, T>(&mut self, getter: F, caller: Location) -> NodeKey
    where
        F: Fn() -> T + 'static,
        T: PartialEq + 'static,
    {
        let inner: Box<RefCell<dyn ComputedOps>> =
            Box::new(RefCell::new(MemoNodeInner::new(Box::new(getter))));

        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Computed(inner),
            ReactiveFlags::NONE,
            Some(self.current_scope.get()),
            caller,
        ));
        self.link_child(node);
        node
    }

    /// Create a new computed node (without equality check)
    pub fn computed_new<F, T>(&mut self, getter: F, caller: Location) -> NodeKey
    where
        F: Fn(Option<T>) -> T + 'static,
        T: 'static,
    {
        let inner: Box<RefCell<dyn ComputedOps>> =
            Box::new(RefCell::new(ComputedNodeInner::new(Box::new(getter))));

        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Computed(inner),
            ReactiveFlags::NONE,
            Some(self.current_scope.get()),
            caller,
        ));
        self.link_child(node);
        node
    }

    /// Track a computed access for reactive dependencies
    pub fn computed_track(this: &ReactiveSystemRef<Self>, node: NodeKey) {
        Self::computed_ensure_fresh(this, node);

        let mut sys = this.borrow_mut();
        if let Some(sub) = sys.active_sub.get() {
            let cycle = sys.cycle;
            sys.link(node, sub, cycle);
        }
    }

    /// Recompute the computed if dirty, pending, or never evaluated, without
    /// subscribing the active subscriber.
    ///
    /// Invariant: must run before reading the value via `value_any()`, which
    /// panics on a never-evaluated node.
    pub fn computed_ensure_fresh(this: &ReactiveSystemRef<Self>, node: NodeKey) {
        let (flags, deps, subs) = {
            let sys = this.borrow();
            let node = sys
                .nodes
                .get(node)
                .expect("Computed accessed after cleanup");
            (node.flags, node.deps, node.subs)
        };

        // Check if dirty or pending
        if flags.contains(ReactiveFlags::DIRTY)
            || (flags.contains(ReactiveFlags::PENDING)
                && (Self::check_dirty(this, deps.unwrap(), node)
                    || {
                        this.borrow_mut().nodes[node]
                            .flags
                            .remove(ReactiveFlags::PENDING);
                        false
                    }))
        {
            if Self::update_computed(this, node) {
                if let Some(subs) = subs {
                    this.borrow_mut().shallow_propagate(subs);
                }
            }
        } else if flags.is_empty() {
            Self::update_computed_inner(this, node);
        }
    }

    /// Get a computed value (cloned, with tracking)
    pub fn computed_get<T>(this: &ReactiveSystemRef<Self>, node: NodeKey) -> T
    where
        T: Clone + 'static,
    {
        Self::computed_track(this, node);
        if let NodeInner::Computed(inner) = &this
            .borrow()
            .nodes
            .get(node)
            .expect("Computed accessed after cleanup")
            .inner
        {
            let guard = inner.borrow();
            guard
                .value_any()
                .downcast_ref::<T>()
                .expect("Computed type mismatch")
                .clone()
        } else {
            panic!("Node is not a Computed");
        }
    }
}
