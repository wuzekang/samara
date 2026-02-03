use crate::system::ReactiveSystemRef;
use crate::types::{EffectNode, Link, NodeInner, NodeKey, ReactiveFlags, ReactiveNode};
use crate::types::{Location, RefCell};
use std::rc::Rc;

impl super::ReactiveSystem {
    /// Create a new effect node
    pub fn new_effect<F: FnMut() + 'static>(
        this: ReactiveSystemRef<Self>,
        effect: F,
        caller: Location,
    ) -> NodeKey {
        let effect = Rc::new(RefCell::new(effect));

        let (prev_scope, prev_sub, node) = {
            let this = this.borrow_mut();
            let parent_scope = this.current_scope.get();

            // Create ONE node that is both the effect AND its scope
            let node = this.nodes.insert(ReactiveNode::new(
                NodeInner::Effect(EffectNode {
                    effect: effect.clone(),
                }),
                ReactiveFlags::WATCHING | ReactiveFlags::RECURSED_CHECK,
                Some(parent_scope),
                caller,
            ));

            // Link this effect/scope node to parent's children list
            this.link_child(node);

            let prev_sub = this.set_active_sub(Some(node));
            if let Some(prev_sub) = prev_sub {
                this.link(node, prev_sub, 0);
            }

            // Set this node as current scope during effect execution
            let prev_scope = this.current_scope.get();
            this.current_scope.set(node);

            (prev_scope, prev_sub, node)
        };

        (effect.borrow_mut())();

        let this = this.borrow_mut();

        // Restore parent scope
        this.current_scope.set(prev_scope);
        this.active_sub.set(prev_sub);
        this.nodes[node].flags.remove(ReactiveFlags::RECURSED_CHECK);
        node
    }

    /// Create a new scope node
    pub fn new_scope<F: FnOnce() + 'static>(
        this: ReactiveSystemRef<Self>,
        f: F,
        caller: Location,
    ) -> NodeKey {
        let (prev_sub, prev_scope, scope_node) = {
            let mut this = this.borrow_mut();
            let parent = this.current_scope.get();
            // Create scope node (effect without execution)
            let scope_node = this.nodes.insert(ReactiveNode::new(
                NodeInner::None,
                ReactiveFlags::NONE,
                Some(parent),
                caller,
            ));

            // Link to parent's children list
            this.link_child(scope_node);

            // Set as current scope
            let prev_scope = this.current_scope.get();
            this.current_scope.set(scope_node);
            let prev_sub = this.set_active_sub(Some(scope_node));

            (prev_sub, prev_scope, scope_node)
        };

        f();

        let this = this.borrow();

        this.set_active_sub(prev_sub);
        this.current_scope.set(prev_scope);

        scope_node
    }

    /// Create a new child scope node with an explicit parent scope
    pub fn new_child_scope(&mut self, parent: NodeKey, caller: Location) -> NodeKey {
        // Create scope node with explicit parent
        let scope_node = self.nodes.insert(ReactiveNode::new(
            NodeInner::None,
            ReactiveFlags::NONE,
            Some(parent),
            caller,
        ));

        // Link to parent's children list
        self.link_child(scope_node);

        scope_node
    }

    /// Run an effect
    pub fn run(this: ReactiveSystemRef<Self>, node: NodeKey) {
        let Some((flags, deps)) = this
            .borrow()
            .nodes
            .get(node)
            .map(|item| (item.flags, item.deps))
        else {
            return;
        };
        if flags.contains(ReactiveFlags::DIRTY)
            || (flags.contains(ReactiveFlags::PENDING)
                && Self::check_dirty(this.clone(), deps.unwrap(), node))
        {
            this.borrow_mut().cycle += 1;
            this.borrow_mut().nodes[node].deps_tail = None;
            this.borrow_mut().nodes[node].flags =
                ReactiveFlags::WATCHING | ReactiveFlags::RECURSED_CHECK;
            Self::cleanup_scope(this.clone(), node);

            // Clean up children from previous execution
            // This prevents memory leaks when effects run multiple times
            this.borrow_mut().purge_child(node);

            let effect = if let NodeInner::Effect(EffectNode { effect }) =
                &this.borrow_mut().nodes[node].inner
            {
                Some(effect.clone())
            } else {
                None
            };

            let prev_sub = this.borrow_mut().set_active_sub(Some(node));
            // Set this node as current scope during effect execution
            let prev_scope = this.borrow_mut().current_scope.get();
            this.borrow_mut().current_scope.set(node);

            if let Some(effect) = effect {
                (effect.borrow_mut())();
            }

            // Restore previous scope
            this.borrow_mut().current_scope.set(prev_scope);
            this.borrow_mut().active_sub.set(prev_sub);

            this.borrow_mut().nodes[node]
                .flags
                .remove(ReactiveFlags::RECURSED_CHECK);
            this.borrow_mut().purge_deps(node, false);
        } else {
            this.borrow_mut().nodes[node].flags = ReactiveFlags::WATCHING;
        }
    }

    /// Trigger a reactive function
    pub fn trigger<F: Fn() + 'static>(this: ReactiveSystemRef<Self>, f: F, caller: Location) {
        // Create a temporary subscriber node
        let parent = this.borrow().current_scope.get();
        let sub = this.borrow_mut().nodes.insert(ReactiveNode::new(
            NodeInner::None,
            ReactiveFlags::WATCHING,
            Some(parent),
            caller,
        ));

        let prev_sub = this.borrow_mut().set_active_sub(Some(sub));
        f();
        this.borrow_mut().active_sub.set(prev_sub);

        // Unlink all dependencies
        let mut current = this.borrow().nodes[sub].deps;
        while let Some(link_key) = current {
            let Link { dep, next_sub, .. } = this.borrow().links[link_key];
            current = next_sub;
            this.borrow_mut().unlink(link_key);

            let subs = this.borrow().nodes[dep].subs;
            if let Some(subs) = subs {
                this.borrow_mut().nodes[sub].flags = ReactiveFlags::NONE;
                this.borrow_mut().propagate(subs);
                this.borrow_mut().shallow_propagate(subs);
            }
        }

        if this.borrow().batch_depth == 0 {
            Self::flush(this.clone());
        }

        // Remove the temporary node
        this.borrow_mut().nodes.remove(sub);
    }

    /// Set the active subscriber
    pub fn set_active_sub(&self, sub: Option<NodeKey>) -> Option<NodeKey> {
        let prev_sub = self.active_sub.get();
        self.active_sub.set(sub);
        prev_sub
    }
}
