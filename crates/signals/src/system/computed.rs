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

    /// Link the active subscriber to `node` (the tracking tail shared by
    /// computed_track and computed_get).
    #[inline]
    pub fn link_active_sub(&mut self, node: NodeKey) {
        if let Some(sub) = self.active_sub.get() {
            let cycle = self.cycle;
            self.link(node, sub, cycle);
        }
    }

    /// Track a computed access for reactive dependencies
    pub fn computed_track(this: &ReactiveSystemRef<Self>, node: NodeKey) {
        Self::computed_ensure_fresh(this, node);

        let mut sys = this.borrow_mut();
        // ensure_fresh may run getters (user code) that dispose this node;
        // linking a removed slot would be an unchecked write.
        sys.nodes.get(node).expect("Computed accessed after cleanup");
        sys.link_active_sub(node);
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
        Self::computed_ensure_fresh(this, node);

        // Value read + subscriber link under ONE system borrow (the old
        // path took two via computed_track). The link runs after the read:
        // no user code sits between, and the liveness expect gates it —
        // linking a node whose scope was disposed inside ensure_fresh's
        // getters would write into a removed slot.
        let mut sys = this.borrow_mut();
        let value = {
            let n = sys
                .nodes
                .get(node)
                .expect("Computed accessed after cleanup");
            match &n.inner {
                NodeInner::Computed(inner) => {
                    let guard = inner.borrow();
                    guard
                        .value_any()
                        .downcast_ref::<T>()
                        .expect("Computed type mismatch")
                        .clone()
                }
                _ => panic!("Node is not a Computed"),
            }
        };
        sys.link_active_sub(node);
        value
    }
}
