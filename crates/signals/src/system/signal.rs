use crate::system::ReactiveSystemRef;
use crate::types::Location;
use crate::{
    flags::ReactiveFlags,
    types::{NodeInner, NodeKey, ReactiveNode, SignalNode},
};

impl super::ReactiveSystem {
    /// Create a new signal node
    pub fn signal_new<T: 'static>(&mut self, initial: T, caller: Location) -> NodeKey {
        use crate::types::BorrowState;
        use std::cell::Cell;
        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Signal(SignalNode {
                value: Box::new(initial),
                borrow_state: Cell::new(BorrowState::Unused),
            }),
            ReactiveFlags::MUTABLE,
            Some(self.current_scope.get()),
            caller,
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
        if self
            .nodes
            .get(node)
            .expect("Signal accessed after cleanup")
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
        let signal = self.signal(node);
        signal.borrow_read_check();
        let value = signal
            .value
            .downcast_ref::<T>()
            .expect("Signal type mismatch")
            .clone();
        signal.release_read();
        value
    }

    /// Read a signal's value and track it as a dependency, all under one
    /// system borrow (signal_track + signal_get fused; the separate calls
    /// cost an extra borrow per read).
    #[inline]
    pub fn signal_get_tracked<T: 'static + Clone>(&mut self, node: NodeKey) -> T {
        // --- track (signal_track body; no user code runs inside) ---
        if self
            .nodes
            .get(node)
            .expect("Signal accessed after cleanup")
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
        // --- read (signal_get body) ---
        let signal = self.signal(node);
        signal.borrow_read_check();
        let value = signal
            .value
            .downcast_ref::<T>()
            .expect("Signal type mismatch")
            .clone();
        signal.release_read();
        value
    }

    /// Notify subscribers of a signal change
    #[inline]
    pub fn signal_notify(this: &ReactiveSystemRef<Self>, node: NodeKey) {
        let subs = {
            let mut sys = this.borrow_mut();
            let node = sys
                .nodes
                .get_mut(node)
                .expect("Signal accessed after cleanup");
            node.flags = ReactiveFlags::MUTABLE | ReactiveFlags::DIRTY;
            node.subs
        };

        if let Some(subs) = subs {
            this.borrow_mut().propagate(subs);
            if this.borrow_mut().batch_depth == 0 {
                Self::flush(this);
            }
        }
    }

    /// Set a signal value WITHOUT notifying subscribers: dependents keep
    /// their stale results until something else invalidates them. Meant for
    /// internal bookkeeping that must not trigger effects.
    pub fn signal_set_silent<T: 'static>(this: &ReactiveSystemRef<Self>, node: NodeKey, value: T) {
        let mut sys = this.borrow_mut();
        let signal = sys.signal(node);
        signal.borrow_write_check();
        *signal
            .value
            .downcast_mut::<T>()
            .expect("Signal type mismatch") = value;
        signal.release_write();
    }

    /// Set a signal value
    #[inline]
    pub fn signal_set<T: 'static>(this: &ReactiveSystemRef<Self>, node: NodeKey, value: T) {
        {
            let mut sys = this.borrow_mut();
            let signal = sys.signal(node);
            signal.borrow_write_check();
            *signal
                .value
                .downcast_mut::<T>()
                .expect("Signal type mismatch") = value;
            signal.release_write();
        }
        Self::signal_notify(this, node);
    }

    /// Update a signal value
    #[inline]
    pub fn signal_update<T: 'static>(
        this: &ReactiveSystemRef<Self>,
        node: NodeKey,
        f: impl FnOnce(&mut T) -> (),
    ) {
        // Swap the value out so `f` runs with NO system borrow held: `f` may
        // re-enter the reactive system (set other signals, dispose scopes...)
        // without aliasing. Box<()> is a ZST, so the placeholder never
        // allocates.
        let mut taken = {
            let mut sys = this.borrow_mut();
            let signal = sys.signal(node);
            signal.borrow_write_check();
            std::mem::replace(&mut signal.value, Box::new(()))
        };

        let slot = taken.downcast_mut::<T>().expect("Signal type mismatch");
        f(slot);

        {
            let mut sys = this.borrow_mut();
            if !sys.nodes.contains_key(node) {
                // The owning scope was disposed inside f; `taken` still owns
                // the value and frees it when dropped on return.
                return;
            }
            let signal = sys.signal(node);
            signal.value = taken;
            signal.release_write();
        }
        Self::signal_notify(this, node);
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
