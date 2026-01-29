use crate::{
    flags::ReactiveFlags,
    types::{NodeInner, NodeKey, ReactiveNode, SignalNode},
};
use std::any::Any;

impl super::ReactiveSystem {
    /// Create a new signal node
    pub fn signal_new<T: 'static>(&mut self, initial: T) -> NodeKey {
        use crate::types::BorrowState;
        use std::cell::Cell;
        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Signal(SignalNode {
                value: Box::leak(Box::new(initial)),
                borrow_state: Cell::new(BorrowState::Unused),
            }),
            ReactiveFlags::MUTABLE,
            Some(self.current_scope.get()),
        ));
        self.link_child(node);
        node
    }

    /// Get a signal from node key
    #[inline]
    pub fn signal(&mut self, node: NodeKey) -> &mut SignalNode {
        let NodeInner::Signal(signal) = &mut self
            .nodes
            .get_mut(node)
            .expect("Signal accessed after cleanup")
            .inner
        else {
            panic!("Node is not a Signal");
        };
        return signal;
    }

    /// Track a signal access for reactive dependencies
    #[inline]
    pub fn signal_track(&mut self, node: NodeKey) {
        if self.nodes[node]
            .flags
            .contains(crate::types::ReactiveFlags::DIRTY)
        {
            self.update_signal(node);
            let subs = self.nodes[node].subs;
            if let Some(subs) = subs {
                self.propagate(subs);
            }
        }
        let mut sub = self.active_sub.get();
        while let Some(sub_key) = sub {
            if self.nodes[sub_key].flags.intersects(
                crate::types::ReactiveFlags::MUTABLE | crate::types::ReactiveFlags::WATCHING,
            ) {
                self.link(node, sub_key, self.cycle);
                break;
            }
            sub = if let Some(deps) = self.nodes[sub_key].subs {
                Some(self.links[deps].sub)
            } else {
                None
            }
        }
    }

    /// Get a signal value (with tracking)
    #[inline]
    pub fn signal_get<T: 'static + Clone>(&mut self, node: NodeKey) -> T {
        self.signal_track(node);
        unsafe { &*(self.signal(node).value as *const dyn Any as *const T) }.clone()
    }

    /// Notify subscribers of a signal change
    #[inline]
    pub fn signal_notify(&mut self, node: NodeKey) {
        let node = &mut self.nodes[node];
        node.flags = ReactiveFlags::MUTABLE | ReactiveFlags::DIRTY;
        let subs = node.subs;
        if let Some(subs) = subs {
            self.propagate(subs);
            if self.batch_depth == 0 {
                self.flush();
            }
        }
    }

    /// Set a signal value
    #[inline]
    pub fn signal_set<T: 'static>(&mut self, node: NodeKey, value: T) {
        let signal = self.signal(node);
        signal.borrow_write_check();
        unsafe { *(signal.value as *mut dyn Any as *mut T) = value };
        signal.release_write();
        self.signal_notify(node);
    }

    /// Update a signal value
    #[inline]
    pub fn signal_update<T: 'static>(&mut self, node: NodeKey, f: impl FnOnce(&mut T) -> ()) {
        let signal = self.signal(node);
        f(unsafe { &mut *(signal.value as *mut dyn Any as *mut T) });
        self.signal_notify(node);
    }

    /// Check if a read borrow is allowed, panic if not
    #[inline]
    pub fn signal_borrow_read_check(&mut self, node: NodeKey) {
        self.signal(node).borrow_read_check();
    }

    /// Check if a write borrow is allowed, panic if not
    #[inline]
    pub fn signal_borrow_write_check(&mut self, node: NodeKey) {
        self.signal(node).borrow_write_check();
    }

    /// Release a read borrow
    #[inline]
    pub fn signal_release_read(&mut self, node: NodeKey) {
        self.signal(node).release_read();
    }

    /// Release a write borrow
    #[inline]
    pub fn signal_release_write(&mut self, node: NodeKey) {
        self.signal(node).release_write();
    }
}
