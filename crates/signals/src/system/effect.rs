use crate::system::ReactiveSystemRef;
use crate::types::{EffectNode, Link, NodeInner, NodeKey, ReactiveFlags, ReactiveNode};
use crate::types::{Location, RefCell};

impl super::ReactiveSystem {
    /// Create a new effect node
    pub fn new_effect<F: FnMut() + 'static>(
        this: &ReactiveSystemRef<Self>,
        effect: F,
        caller: Location,
    ) -> NodeKey {
        Self::new_effect_inner(this, effect, caller, ReactiveFlags::NONE)
    }

    /// Like [`new_effect`], but the effect re-runs until the graph settles
    /// when its body writes a signal it depends on.
    pub fn new_effect_cascade<F: FnMut() + 'static>(
        this: &ReactiveSystemRef<Self>,
        effect: F,
        caller: Location,
    ) -> NodeKey {
        Self::new_effect_inner(this, effect, caller, ReactiveFlags::CASCADE)
    }

    fn new_effect_inner<F: FnMut() + 'static>(
        this: &ReactiveSystemRef<Self>,
        effect: F,
        caller: Location,
        extra_flags: ReactiveFlags,
    ) -> NodeKey {
        // The closure stays local for the initial run; the node is created
        // with inner == None and receives it afterwards, so the initial run
        // needs no take/restore round-trip at all.
        let effect: Box<RefCell<F>> = Box::new(RefCell::new(effect));

        let (prev_scope, prev_sub, node) = {
            let mut sys = this.borrow_mut();
            let parent_scope = sys.current_scope.get();

            // Create ONE node that is both the effect AND its scope
            let node = sys.nodes.insert(ReactiveNode::new(
                NodeInner::None,
                ReactiveFlags::WATCHING | ReactiveFlags::RECURSED_CHECK | extra_flags,
                Some(parent_scope),
                caller,
            ));

            // Link this effect/scope node to parent's children list
            sys.link_child(node);

            let prev_sub = sys.set_active_sub(Some(node));
            if let Some(prev_sub) = prev_sub {
                sys.link(node, prev_sub, 0);
            }

            // Set this node as current scope during effect execution
            let prev_scope = sys.current_scope.get();
            sys.current_scope.set(node);

            (prev_scope, prev_sub, node)
        };

        // Initial run: user code — no system borrow held.
        (effect.borrow_mut())();

        {
            let mut sys = this.borrow_mut();
            sys.current_scope.set(prev_scope);
            sys.active_sub.set(prev_sub);
            if let Some(n) = sys.nodes.get_mut(node) {
                n.inner = NodeInner::Effect(EffectNode { effect });
                n.flags.remove(ReactiveFlags::RECURSED_CHECK);
            }
            // else: the effect disposed itself during the initial run;
            // `effect` frees the closure when dropped.
        }

        // Self-recursion (opt-in CASCADE): the initial body wrote a signal
        // this effect depends on — continue running until the graph settles.
        if this.borrow().nodes.get(node).is_some_and(|n| {
            n.flags
                .contains(ReactiveFlags::RECURSED | ReactiveFlags::CASCADE)
        }) {
            {
                let mut sys = this.borrow_mut();
                if let Some(n) = sys.nodes.get_mut(node) {
                    n.flags = ReactiveFlags::DIRTY
                        | ReactiveFlags::WATCHING
                        | ReactiveFlags::RECURSED_CHECK
                        | ReactiveFlags::CASCADE;
                }
            }
            Self::run(this, node);
        }
        node
    }

    /// Create a new scope node
    pub fn new_scope<F: FnOnce() + 'static>(
        this: &ReactiveSystemRef<Self>,
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
        self.nodes
            .get(parent)
            .expect("Scope accessed after cleanup");
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
    pub fn run(this: &ReactiveSystemRef<Self>, node: NodeKey) {
        loop {
            let flags_deps = {
                let sys = this.borrow();
                sys.nodes.get(node).map(|item| (item.flags, item.deps))
            };
            let Some((flags, deps)) = flags_deps else {
                return;
            };
            if flags.contains(ReactiveFlags::DIRTY)
                || (flags.contains(ReactiveFlags::PENDING)
                    && Self::check_dirty(this, deps.unwrap(), node))
            {
            let taken = {
                let mut sys = this.borrow_mut();
                // The effect may have been disposed inside check_dirty's
                // getters (user code) — bail out instead of touching a
                // removed slot.
                if !sys.nodes.contains_key(node) {
                    return;
                }
                // Grab the closure FIRST: a nested flush may re-enter this
                // node while its body is in flight (inner == None). Skip in
                // that case — the in-flight run settles the pending state.
                let mut taken = None;
                if let Some(n) = sys.nodes.get_mut(node) {
                    taken = n.take_effect();
                }
                let Some(effect) = taken else {
                    return;
                };
                sys.cycle += 1;
                sys.nodes[node].deps_tail = None;
                // Preserve opt-in flags (CASCADE) across re-runs.
                sys.nodes[node].flags = ReactiveFlags::WATCHING
                    | ReactiveFlags::RECURSED_CHECK
                    | (sys.nodes[node].flags & ReactiveFlags::CASCADE);
                Some(effect)
            };

            // cleanup_scope runs user cleanup callbacks — no borrow held inside.
            Self::cleanup_scope(this, node);

            // Clean up children from previous execution
            // This prevents memory leaks when effects run multiple times
            let (taken, prev_scope, prev_sub) = {
                let mut sys = this.borrow_mut();
                sys.purge_child(node);
                let prev_sub = sys.set_active_sub(Some(node));
                // Set this node as current scope during effect execution
                let prev_scope = sys.current_scope.get();
                sys.current_scope.set(node);
                (taken, prev_scope, prev_sub)
            };

            // Effect body runs here (user code) — no system borrow held.
            if let Some(effect) = &taken {
                (effect.borrow_mut())();
            }

            // Restore previous scope
            {
                let mut sys = this.borrow_mut();
                sys.current_scope.set(prev_scope);
                sys.active_sub.set(prev_sub);
                if let Some(n) = sys.nodes.get_mut(node) {
                    if let Some(effect) = taken {
                        n.inner = NodeInner::Effect(EffectNode { effect });
                    }
                    n.flags.remove(ReactiveFlags::RECURSED_CHECK);
                }
                // else: the effect disposed itself; `taken` frees the closure.
                sys.purge_deps(node, false);
            }

            // Self-recursion (opt-in CASCADE): the body wrote a signal this
            // effect depends on (flagged RECURSED during propagation). Run
            // again until the graph settles.
            let recursed = {
                let mut sys = this.borrow_mut();
                match sys.nodes.get_mut(node) {
                    Some(n)
                        if n.flags
                            .contains(ReactiveFlags::RECURSED | ReactiveFlags::CASCADE) =>
                    {
                        n.flags = ReactiveFlags::DIRTY
                            | ReactiveFlags::WATCHING
                            | ReactiveFlags::RECURSED_CHECK
                            | ReactiveFlags::CASCADE;
                        true
                    }
                    _ => false,
                }
            };
            if recursed {
                continue;
            }
            return;
        } else {
            let mut sys = this.borrow_mut();
            sys.nodes[node].flags =
                ReactiveFlags::WATCHING | (sys.nodes[node].flags & ReactiveFlags::CASCADE);
            return;
        }
        }
    }

    /// Trigger a reactive function
    pub fn trigger<F: Fn() + 'static>(this: &ReactiveSystemRef<Self>, f: F, caller: Location) {
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
            let Link { dep, next_dep, .. } = this.borrow().links[link_key];
            current = next_dep;
            this.borrow_mut().unlink(link_key);

            let subs = this.borrow().nodes[dep].subs;
            if let Some(subs) = subs {
                this.borrow_mut().nodes[sub].flags = ReactiveFlags::NONE;
                this.borrow_mut().propagate(subs);
                this.borrow_mut().shallow_propagate(subs);
            }
        }

        if this.borrow().batch_depth == 0 {
            Self::flush(this);
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
