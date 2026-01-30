use crate::types::{EffectNode, Link, NodeInner, NodeKey, ReactiveFlags, ReactiveNode};

impl super::ReactiveSystem {
    /// Create a new effect node
    pub fn new_effect<F: Fn() + 'static>(&mut self, effect: F) -> NodeKey {
        let parent_scope = self.current_scope.get();

        // Create ONE node that is both the effect AND its scope
        let node = self.nodes.insert(ReactiveNode::new(
            NodeInner::Effect(EffectNode {
                effect: Box::new(effect),
            }),
            crate::types::ReactiveFlags::WATCHING | crate::types::ReactiveFlags::RECURSED_CHECK,
            Some(parent_scope),
        ));

        // Link this effect/scope node to parent's children list
        self.link_child(node);

        let prev_sub = self.set_active_sub(Some(node));
        if let Some(prev_sub) = prev_sub {
            self.link(node, prev_sub, 0);
        }

        // Set this node as current scope during effect execution
        let prev_scope = self.current_scope.get();
        self.current_scope.set(node);

        let NodeInner::Effect(effect) = &self.nodes[node].inner else {
            panic!("Node is not an Effect");
        };

        (effect.effect)();

        // Restore parent scope
        self.current_scope.set(prev_scope);
        self.active_sub.set(prev_sub);

        self.nodes[node]
            .flags
            .remove(crate::types::ReactiveFlags::RECURSED_CHECK);

        node
    }

    /// Create a new scope node
    pub fn new_scope<F: FnOnce() + 'static>(&mut self, f: F) -> NodeKey {
        let parent = self.current_scope.get();
        // Create scope node (effect without execution)
        let scope_node = self.nodes.insert(ReactiveNode::new(
            NodeInner::None,
            crate::types::ReactiveFlags::NONE,
            Some(parent),
        ));

        // Link to parent's children list
        self.link_child(scope_node);

        // Set as current scope
        let prev_scope = self.current_scope.get();
        self.current_scope.set(scope_node);
        let prev_sub = self.set_active_sub(Some(scope_node));

        f();
        
        self.set_active_sub(prev_sub);
        self.current_scope.set(prev_scope);

        scope_node
    }

    /// Create a new child scope node with an explicit parent scope
    pub fn new_child_scope(&mut self, parent: NodeKey) -> NodeKey {
        // Create scope node with explicit parent
        let scope_node = self.nodes.insert(ReactiveNode::new(
            NodeInner::None,
            ReactiveFlags::NONE,
            Some(parent),
        ));

        // Link to parent's children list
        self.link_child(scope_node);

        scope_node
    }

    /// Run an effect
    pub fn run(&mut self, node: NodeKey) {
        let Some(n) = self.nodes.get(node) else {
            return;
        };
        let flags = n.flags;
        if flags.contains(crate::types::ReactiveFlags::DIRTY)
            || (flags.contains(crate::types::ReactiveFlags::PENDING)
                && self.check_dirty(self.nodes[node].deps.unwrap(), node))
        {
            self.cycle += 1;
            self.nodes[node].deps_tail = None;
            self.nodes[node].flags =
                crate::types::ReactiveFlags::WATCHING | crate::types::ReactiveFlags::RECURSED_CHECK;

            self.cleanup_scope(node);

            // Clean up children from previous execution
            // This prevents memory leaks when effects run multiple times
            self.purge_child(node);

            let NodeInner::Effect(EffectNode { effect }) = &self.nodes[node].inner else {
                panic!("Node is not an Effect");
            };

            let prev_sub = self.set_active_sub(Some(node));
            // Set this node as current scope during effect execution
            let prev_scope = self.current_scope.get();
            self.current_scope.set(node);

            (effect)();

            // Restore previous scope
            self.current_scope.set(prev_scope);
            self.active_sub.set(prev_sub);

            self.nodes[node]
                .flags
                .remove(crate::types::ReactiveFlags::RECURSED_CHECK);
            self.purge_deps(node, false);
        } else {
            self.nodes[node].flags = crate::types::ReactiveFlags::WATCHING;
        }
    }

    /// Trigger a reactive function
    pub fn trigger<F: Fn() + 'static>(&mut self, f: F) {
        // Create a temporary subscriber node
        let parent = self.current_scope.get();
        let sub = self.nodes.insert(ReactiveNode::new(
            NodeInner::None,
            crate::types::ReactiveFlags::WATCHING,
            Some(parent),
        ));

        let prev_sub = self.set_active_sub(Some(sub));
        f();
        self.active_sub.set(prev_sub);

        // Unlink all dependencies
        let mut current = self.nodes[sub].deps;
        while let Some(link_key) = current {
            let Link { dep, next_sub, .. } = self.links[link_key];
            current = next_sub;
            self.unlink(link_key);

            let subs = self.nodes[dep].subs;
            if let Some(subs) = subs {
                self.nodes[sub].flags = crate::types::ReactiveFlags::NONE;
                self.propagate(subs);
                self.shallow_propagate(subs);
            }
        }

        if self.batch_depth == 0 {
            self.flush();
        }

        // Remove the temporary node
        self.nodes.remove(sub);
    }

    /// Set the active subscriber
    pub fn set_active_sub(&self, sub: Option<NodeKey>) -> Option<NodeKey> {
        let prev_sub = self.active_sub.get();
        self.active_sub.set(sub);
        prev_sub
    }
}
