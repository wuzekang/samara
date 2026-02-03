use crate::system::ReactiveSystemRef;
use crate::{
    flags::ReactiveFlags,
    types::{NodeInner, NodeKey},
};

impl super::ReactiveSystem {
    /// Link a child node to its parent's children list
    pub fn link_child(&mut self, child: NodeKey) {
        let parent = match self.nodes[child].parent {
            Some(p) => p,
            None => return, // Root node has no parent to link to
        };

        // Add to parent's children linked list (insert at head)
        let head = self.nodes[parent].child;
        if let Some(head) = head {
            self.nodes[head].prev = Some(child);
        }
        self.nodes[child].next = head;
        self.nodes[child].prev = None;
        self.nodes[parent].child = Some(child);
    }

    /// Unlink a child node from its parent's children list
    pub fn unlink_child(&mut self, child: NodeKey) {
        let parent = self.nodes[child].parent;
        let prev = self.nodes[child].prev;
        let next = self.nodes[child].next;

        if let Some(prev) = prev {
            self.nodes[prev].next = next;
        }
        if let Some(next) = next {
            self.nodes[next].prev = prev;
        }

        if let Some(parent) = parent {
            if self.nodes[parent].child == Some(child) {
                self.nodes[parent].child = next;
            }
        }

        self.nodes[child].prev = None;
        self.nodes[child].next = None;
    }

    /// Update a computed node and return whether it changed
    pub fn update_computed(this: ReactiveSystemRef<Self>, node: NodeKey) -> bool {
        this.borrow_mut().nodes[node].deps_tail = None;
        let dirty = Self::update_computed_inner(this.clone(), node);
        dirty
    }

    pub fn update_computed_inner(this: ReactiveSystemRef<Self>, node: NodeKey) -> bool {
        this.borrow_mut().cycle += 1;
        this.borrow_mut().nodes[node].flags =
            ReactiveFlags::MUTABLE | ReactiveFlags::RECURSED_CHECK;
        let prev_sub = this.borrow_mut().set_active_sub(Some(node));

        let inner = if let NodeInner::Computed(inner) = &this.borrow_mut().nodes[node].inner {
            Some(inner.clone())
        } else {
            None
        };
        let dirty = if let Some(inner) = inner {
            inner.borrow_mut().update()
        } else {
            false
        };

        this.borrow_mut().nodes[node]
            .flags
            .remove(ReactiveFlags::RECURSED_CHECK);
        this.borrow_mut().active_sub.set(prev_sub);
        this.borrow_mut().purge_deps(node, false);

        dirty
    }

    /// Mark a signal node as mutable
    #[inline]
    pub fn update_signal(&mut self, s: NodeKey) {
        self.nodes[s].flags = ReactiveFlags::MUTABLE;
    }

    /// Update a node (computed or signal) and return whether it changed
    #[inline]
    pub fn update(this: ReactiveSystemRef<Self>, node: NodeKey) -> bool {
        if this.borrow().nodes[node].deps_tail.is_some() {
            Self::update_computed(this, node)
        } else {
            this.borrow_mut().update_signal(node);
            true
        }
    }

    pub fn cleanup_scope(this: ReactiveSystemRef<Self>, node: NodeKey) {
        let mut current = this.borrow().nodes[node].child;
        while let Some(child) = current {
            current = this.borrow().nodes[child].next;
            if match this.borrow().nodes[child].inner {
                NodeInner::Effect(_) | NodeInner::None => true,
                _ => false,
            } {
                Self::cleanup_scope(this.clone(), child)
            }
        }
        if let Some(cleanups) = { this.borrow_mut().cleanups.remove(node) } {
            for cleanup in cleanups.into_iter().rev() {
                cleanup();
            }
        }
    }

    /// Cleanup children of a node
    pub fn purge_child(&mut self, node: NodeKey) {
        let mut current = self.nodes[node].child;
        while let Some(child) = current {
            current = self.nodes[child].next;

            match self.nodes[child].inner {
                NodeInner::Effect(_) | NodeInner::None => {
                    self.purge_scope(child);
                }
                NodeInner::Computed(_) | NodeInner::Signal(_) => {
                    self.purge_node(child);
                }
            }

            self.nodes.remove(child);
        }

        self.nodes[node].child = None;
    }

    /// Cleanup an scope node
    pub fn purge_scope(&mut self, node: NodeKey) {
        self.purge_child(node);

        self.nodes[node].deps_tail = None;
        self.nodes[node].flags = ReactiveFlags::NONE;
        self.purge_deps(node, false);

        let subs = self.nodes[node].subs;
        if let Some(subs) = subs {
            let _ = self.unlink(subs);
        }
    }

    /// Remove all links from a node (idempotent)
    pub fn purge_node(&mut self, node: NodeKey) {
        // Purge all dependency links (to avoid accessing removed child nodes later)
        self.purge_deps(node, true);

        // Purge all subscriber links (links FROM other nodes TO this node)
        // This is critical to prevent accessing already-deleted nodes during unlink()
        self.purge_subs(node);
    }

    /// Fully dispose a node (cleanup and remove)
    pub fn dispose_scope(this: ReactiveSystemRef<Self>, node: NodeKey) {
        if !this.borrow().nodes.contains_key(node) {
            return;
        }
        Self::cleanup_scope(this.clone(), node);
        this.borrow_mut().purge_scope(node);
        this.borrow_mut().unlink_child(node);
        this.borrow_mut().contexts.remove(node);
        this.borrow_mut().nodes.remove(node);
    }

    pub fn cleanup(this: ReactiveSystemRef<Self>) {
        let node = this.borrow().root;
        Self::cleanup_scope(this.clone(), node);
        this.borrow_mut().purge_scope(node);
        this.borrow_mut().unlink_child(node);
    }
}
