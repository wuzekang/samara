use crate::system::ReactiveSystemRef;
use crate::types::{
    ComputedNodeInner, ComputedOps, MemoNodeInner, NodeInner, NodeKey, ReactiveFlags, ReactiveNode,
};
use crate::types::{Location, RefCell};
use std::rc::Rc;

impl super::ReactiveSystem {
    /// Create a new memo node (with equality check)
    pub fn computed_memo<F, T>(&mut self, getter: F, caller: Location) -> NodeKey
    where
        F: Fn() -> T + 'static,
        T: PartialEq + 'static,
    {
        let inner = Rc::new(RefCell::new(MemoNodeInner::new(Box::new(getter))));

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
        let inner = Rc::new(RefCell::new(ComputedNodeInner::new(Box::new(getter))));

        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Computed(inner as Rc<RefCell<dyn ComputedOps>>),
            ReactiveFlags::NONE,
            Some(self.current_scope.get()),
            caller,
        ));
        self.link_child(node);
        node
    }

    /// Track a computed access for reactive dependencies
    pub fn computed_track(this: ReactiveSystemRef<Self>, node: NodeKey) {
        let ReactiveNode {
            flags, deps, subs, ..
        } = this.borrow().nodes[node];

        // Check if dirty or pending
        if flags.contains(ReactiveFlags::DIRTY)
            || (flags.contains(ReactiveFlags::PENDING)
                && (Self::check_dirty(this.clone(), deps.unwrap(), node))
                || {
                    this.borrow_mut().nodes[node]
                        .flags
                        .remove(ReactiveFlags::PENDING);
                    false
                })
        {
            if Self::update_computed(this.clone(), node) {
                if let Some(subs) = subs {
                    this.borrow_mut().shallow_propagate(subs);
                }
            }
        } else if flags.is_empty() {
            Self::update_computed_inner(this.clone(), node);
        }

        let sub = this.borrow_mut().active_sub.get();
        if let Some(sub) = sub {
            let cycle = this.borrow().cycle;
            this.borrow_mut().link(node, sub, cycle);
        }
    }

    /// Get a computed value (cloned, with tracking)
    pub fn computed_get<T>(this: ReactiveSystemRef<Self>, node: NodeKey) -> T
    where
        T: Clone + 'static,
    {
        Self::computed_track(this.clone(), node);
        if let NodeInner::Computed(inner) = &this.borrow().nodes[node].inner {
            unsafe { &*(inner.borrow().as_any() as *const dyn std::any::Any as *const T) }.clone()
        } else {
            panic!("Node is not a Computed");
        }
    }
}
