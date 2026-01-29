use crate::types::{
    ComputedNodeInner, ComputedOps, MemoNodeInner, NodeInner, NodeKey, ReactiveFlags, ReactiveNode,
};

impl super::ReactiveSystem {
    /// Create a new memo node (with equality check)
    pub fn computed_memo<F, T>(&mut self, getter: F) -> NodeKey
    where
        F: Fn() -> T + 'static,
        T: PartialEq + 'static,
    {
        let inner: Box<dyn ComputedOps> = Box::new(MemoNodeInner::new(Box::new(getter)));

        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Computed(inner),
            ReactiveFlags::NONE,
            Some(self.current_scope.get()),
        ));
        self.link_child(node);
        node
    }

    /// Create a new computed node (without equality check)
    pub fn computed_new<F, T>(&mut self, getter: F) -> NodeKey
    where
        F: Fn() -> T + 'static,
        T: 'static,
    {
        let inner: Box<dyn ComputedOps> = Box::new(ComputedNodeInner::new(Box::new(getter)));

        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Computed(inner),
            ReactiveFlags::NONE,
            Some(self.current_scope.get()),
        ));
        self.link_child(node);
        node
    }

    /// Track a computed access for reactive dependencies
    pub fn computed_track(&mut self, node: NodeKey) {
        let ReactiveNode {
            flags, deps, subs, ..
        } = self.nodes[node];

        // Check if dirty or pending
        if flags.contains(ReactiveFlags::DIRTY)
            || (flags.contains(ReactiveFlags::PENDING) && self.check_dirty(deps.unwrap(), node))
        {
            if self.update_computed(node) {
                if let Some(subs) = subs {
                    self.shallow_propagate(subs);
                }
            }
        } else if flags.is_empty() {
            // First access - compute initial value
            self.cycle += 1;
            let prev_sub = self.set_active_sub(Some(node));
            let n = &mut self.nodes[node];
            n.flags = ReactiveFlags::MUTABLE | ReactiveFlags::RECURSED_CHECK;
            if let NodeInner::Computed(inner) = &mut n.inner {
                inner.update();
            }
            self.active_sub.set(prev_sub);
            n.flags.remove(ReactiveFlags::RECURSED_CHECK);
            self.purge_deps(node, false);
        }

        // Link to active subscriber (after computing value)
        let mut sub = self.active_sub.get();
        while let Some(sub_key) = sub {
            let ReactiveNode { flags, subs, .. } = self.nodes[sub_key];
            if flags.intersects(ReactiveFlags::MUTABLE | ReactiveFlags::WATCHING) {
                self.link(node, sub_key, self.cycle);
                break;
            }
            sub = if let Some(deps) = subs {
                Some(self.links[deps].sub)
            } else {
                None
            }
        }
    }

    /// Peek at a computed value without tracking
    #[inline]
    pub fn computed_peek<T>(&mut self, node: NodeKey) -> &T
    where
        T: 'static,
    {
        if let NodeInner::Computed(inner) = &self.nodes[node].inner {
            unsafe { &*(inner.as_any() as *const dyn std::any::Any as *const T) }
        } else {
            panic!("Node is not a Computed");
        }
    }

    /// Read a computed value (with tracking)
    pub fn computed_read<T>(&mut self, node: NodeKey) -> &T
    where
        T: 'static,
    {
        self.computed_track(node);
        self.computed_peek(node)
    }

    /// Get a computed value (cloned, with tracking)
    pub fn computed_get<T>(&mut self, node: NodeKey) -> T
    where
        T: Clone + 'static,
    {
        self.computed_track(node);
        if let NodeInner::Computed(inner) = &self.nodes[node].inner {
            unsafe { &*(inner.as_any() as *const dyn std::any::Any as *const T) }.clone()
        } else {
            panic!("Node is not a Computed");
        }
    }
}
