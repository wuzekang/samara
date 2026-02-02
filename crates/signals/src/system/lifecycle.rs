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
    pub fn update_computed(&mut self, node: NodeKey) -> bool {
        let prev_sub = self.set_active_sub(Some(node));
        self.cycle += 1;
        self.nodes[node].deps_tail = None;
        self.nodes[node].flags = ReactiveFlags::MUTABLE | ReactiveFlags::RECURSED_CHECK;
        let dirty = if let NodeInner::Computed(inner) = &mut self.nodes[node].inner {
            inner.update()
        } else {
            false
        };
        self.active_sub.set(prev_sub);
        self.purge_deps(node, false);
        self.nodes[node].flags.remove(ReactiveFlags::RECURSED_CHECK);
        dirty
    }

    /// Mark a signal node as mutable
    #[inline]
    pub fn update_signal(&mut self, s: NodeKey) {
        self.nodes[s].flags = ReactiveFlags::MUTABLE;
    }

    /// Update a node (computed or signal) and return whether it changed
    #[inline]
    pub fn update(&mut self, node: NodeKey) -> bool {
        if self.nodes[node].deps_tail.is_some() {
            self.update_computed(node)
        } else {
            self.update_signal(node);
            true
        }
    }

    pub fn cleanup_scope(&mut self, node: NodeKey) {
        let mut current = self.nodes[node].child;
        while let Some(child) = current {
            current = self.nodes[child].next;
            match self.nodes[child].inner {
                NodeInner::Effect(_) | NodeInner::None => {
                    self.cleanup_scope(child);
                }
                _ => {}
            }
        }
        if let Some(cleanups) = self.cleanups.remove(node) {
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
    pub fn dispose_scope(&mut self, node: NodeKey) {
        if !self.nodes.contains_key(node) {
            return;
        }
        self.cleanup_scope(node);
        self.purge_scope(node);
        self.unlink_child(node);
        self.contexts.remove(node);
        self.nodes.remove(node);
    }

    pub fn cleanup(&mut self) {
        let node = self.root;
        self.cleanup_scope(node);
        self.purge_scope(node);
        self.unlink_child(node);
    }
}
