use samara_signals::*;

#[test]
fn test_count_signal() {
    let (nodes_before, links_before) = count();
    let _sig = signal(42);
    let (nodes_after, links_after) = count();

    // Creating a signal should create exactly 1 node, 0 links
    assert_eq!(nodes_after - nodes_before, 1);
    assert_eq!(links_after - links_before, 0);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_computed() {
    let (nodes_before, links_before) = count();
    let sig = signal(10);
    let (nodes_after_sig, links_after_sig) = count();

    // Signal creation: 1 node, 0 links
    assert_eq!(nodes_after_sig - nodes_before, 1);
    assert_eq!(links_after_sig - links_before, 0);

    let comp = memo(move || sig.get() * 2);
    let (nodes_after_comp, links_after_comp) = count();

    // Computed creation: 1 node, 0 links (link not created until get() is called)
    assert_eq!(nodes_after_comp - nodes_after_sig, 1);
    assert_eq!(links_after_comp - links_after_sig, 0);

    // First access to computed creates the link
    let _val = comp.get();
    let (nodes_after, links_after) = count();

    // After get(): 1 link (sig -> comp) is created
    assert_eq!(links_after - links_after_comp, 1);

    // Total: signal (1) + computed (1) = 2 new nodes, 1 new link
    assert_eq!(nodes_after - nodes_before, 2);
    assert_eq!(links_after - links_before, 1);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_effect() {
    let (nodes_before, links_before) = count();
    let sig = signal(1);
    let (nodes_after_sig, links_after_sig) = count();

    // Signal creation: 1 node, 0 links
    assert_eq!(nodes_after_sig - nodes_before, 1);
    assert_eq!(links_after_sig - links_before, 0);

    let _eff = effect(move || {
        let _ = sig.get();
    });
    let (nodes_after, links_after) = count();

    // Effect creation: 1 node, 1 link (sig -> eff) when it accesses signal
    assert_eq!(nodes_after - nodes_after_sig, 1);
    assert_eq!(links_after - links_after_sig, 1);

    // Total: signal (1) + effect (1) = 2 new nodes, 1 new link
    assert_eq!(nodes_after - nodes_before, 2);
    assert_eq!(links_after - links_before, 1);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_scope() {
    let (nodes_before, links_before) = count();

    let scope = scope(|| {
        let _sig = signal(100);
        let _comp = memo(|| 50);
    });
    let (nodes_after, links_after) = count();

    // scope adds 1 scope node + 2 child nodes (signal + computed)
    // No links created since computed doesn't access signal
    let nodes_delta = nodes_after - nodes_before;
    let links_delta = links_after - links_before;

    assert_eq!(nodes_delta, 3);
    assert_eq!(links_delta, 0);

    // After cleanup, nodes should decrease
    scope.dispose();
    let (nodes_after_cleanup, links_after_cleanup) = count();

    assert_eq!(nodes_after - nodes_after_cleanup, 3);
    assert_eq!(links_after - links_after_cleanup, 0);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_with_links() {
    let (nodes_before, links_before) = count();

    let sig = signal(5);
    let (nodes_after_sig, links_after_sig) = count();

    // Signal creation: 1 node, 0 links
    assert_eq!(nodes_after_sig - nodes_before, 1);
    assert_eq!(links_after_sig - links_before, 0);

    let comp = memo(move || sig.get() + 1);
    let (nodes_after_comp, links_after_comp) = count();

    // Computed creation: 1 node, 0 links (link not created until get())
    assert_eq!(nodes_after_comp - nodes_after_sig, 1);
    assert_eq!(links_after_comp - links_after_sig, 0);

    let _eff = effect(move || {
        let _ = comp.get();
    });
    let (nodes_after_eff, links_after_eff) = count();

    // Effect creation: 1 node
    assert_eq!(nodes_after_eff - nodes_after_comp, 1);

    // When effect runs, it calls comp.get() which:
    // 1. Sets computed as active_sub
    // 2. Accesses sig.get(), creating sig -> comp link
    // 3. Returns to effect as active_sub
    // 4. Effect calls comp.get(), creating comp -> eff link
    // So 2 links are created: sig -> comp and comp -> eff
    assert_eq!(links_after_eff - links_after_comp, 2);
    assert_eq!(links_after_eff - links_before, 2);

    // Trigger the effect to re-establish links
    sig.set(10);

    let (nodes_after, links_after) = count();

    // After triggering, nodes and links stay the same
    assert_eq!(nodes_after - nodes_before, 3);
    assert_eq!(links_after - links_before, 2);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_cleanup_reduces_nodes() {
    let (nodes_before, links_before) = count();

    let scope = scope(|| {
        let _s1 = signal(1);
        let _s2 = signal(2);
        let _c1 = memo(|| 3);
        let _c2 = memo(|| 4);
    });
    let (nodes_with_scope, links_with_scope) = count();

    // Inside scope: 1 scope node + 2 signals + 2 computed = 5 nodes
    // No links since computed don't access signals
    assert_eq!(nodes_with_scope - nodes_before, 5);
    assert_eq!(links_with_scope - links_before, 0);

    scope.dispose();
    let (nodes_after, links_after) = count();

    // After cleanup, should return to previous state
    assert_eq!(nodes_with_scope - nodes_after, 5);
    assert_eq!(links_with_scope - links_after, 0);
    assert_eq!(nodes_after, nodes_before);
    assert_eq!(links_after, links_before);

    cleanup();
    assert_eq!(count(), (1, 0));
}

#[test]
fn test_count_cleanup() {
    let _s0 = signal(0);

    effect(move || {
        _s0.get();
    });

    _s0.set(1);

    let (nodes_before, links_before) = count();

    let scope = scope(|| {
        let _s1 = signal(1);
        let _s2 = signal(2);
        let _s3 = signal(3);
        let _c1 = memo(move || _s1.get() + _s2.get());
        let _c2 = memo(move || _c1.get() + _s3.get());
    });

    scope.dispose();

    let (nodes_after, links_after) = count();

    assert_eq!(nodes_after, nodes_before);
    assert_eq!(links_after, links_before);
}

#[test]
fn test_count_effect_run() {
    let (nodes_before, links_before) = count();
    let n = 5;

    for _ in 0..n {
        let trigger = signal(0);

        let e = scope(move || {
            effect(move || {
                trigger.get();
                let _s1 = signal(1);
                let _s2 = signal(2);
                let _c1 = memo(move || _s1.get() + _s2.get());
            });
        });

        for v in [2, 3, 4] {
            trigger.set(v);
        }

        e.dispose();
    }

    let (nodes_after, links_after) = count();

    assert_eq!(nodes_after, nodes_before + n);
    assert_eq!(links_after, links_before);
}

#[test]
fn test_count_nested_effect_with_cleanup_no_leak() {
    // Test based on nested_effect_cleanup.rs::test_nested_effect_with_cleanup
    // Verify that creating and destroying inner effects doesn't leak nodes/links

    // Start from clean state
    let (nodes_before, links_before) = count();

    let sig = signal(1);
    let (nodes_after_sig, links_after_sig) = count();

    // Signal: 1 node, 0 links
    assert_eq!(
        nodes_after_sig - nodes_before,
        1,
        "Signal should add 1 node"
    );
    assert_eq!(
        links_after_sig - links_before,
        0,
        "Signal should add 0 links"
    );

    let outer = effect({
        move || {
            let _v = sig.get();

            // Create inner effect
            let inner = effect(move || {
                // inner effect doesn't subscribe to anything
            });

            // Immediately cleanup inner effect
            inner.dispose();
        }
    });

    let (nodes_after_outer, links_after_outer) = count();

    // (outer effect + inner effect): 2 node + 1 link (sig -> outer)
    assert_eq!(nodes_after_outer - nodes_after_sig, 1);
    assert_eq!(
        links_after_outer - links_after_sig,
        1,
        "Outer effect should add 1 link"
    );

    // Trigger outer effect to run again
    sig.set(2);

    let (nodes_after, links_after) = count();

    // After re-run: should still be 1 signal + 1 outer effect
    // inner effect was cleaned up, so no additional nodes
    assert_eq!(
        nodes_after - nodes_before,
        2,
        "Should have sig + outer nodes"
    );
    assert_eq!(
        links_after - links_before,
        1,
        "Should have sig -> outer link"
    );

    outer.dispose();
    let (nodes_final, links_final) = count();

    // After cleanup: should return to initial state + sig (which is not a child of outer)
    assert_eq!(
        nodes_final,
        nodes_before + 1,
        "Node count should be initial state + sig"
    );
    assert_eq!(
        links_final, links_before,
        "Link count should return to initial state"
    );
}

#[test]
fn test_count_effect_creates_and_destroys_inner_no_leak() {
    // Test based on nested_effect_cleanup.rs::test_effect_creates_and_destroys_inner_effect
    // Verify that repeatedly creating/destroying inner effects doesn't accumulate
    let (nodes_before, links_before) = count();

    let sig = signal(1);
    let (nodes_after_sig, _) = count();

    // Signal: 1 node
    assert_eq!(nodes_after_sig - nodes_before, 1);

    let outer = effect(move || {
        let _v = sig.get();

        // Create inner effect
        let inner = effect(move || {
            // inner doesn't subscribe to anything
        });

        // Immediately cleanup
        inner.dispose();
    });

    let (nodes_after_outer, _) = count();

    // outer effect + signal + inner
    assert_eq!(nodes_after_outer - nodes_after_sig, 1);

    // Trigger outer effect multiple times
    sig.set(2);
    sig.set(3);
    sig.set(4);

    let (nodes_after, links_after) = count();

    // Should still only have signal + outer effect (inner effects were cleaned up)
    assert_eq!(nodes_after - nodes_before, 2); // sig + outer
    assert_eq!(links_after - links_before, 1); // sig -> outer

    outer.dispose();
    let (nodes_final, links_final) = count();

    // Final state: initial state + sig (sig is not a child of outer)
    assert_eq!(
        nodes_final,
        nodes_before + 1,
        "Should have initial nodes + sig"
    );
    assert_eq!(
        links_final, links_before,
        "Link count should match initial state"
    );
}

#[test]
fn test_count_nested_scope_cleanup_no_leak() {
    // Test based on nested_effect_cleanup.rs::test_nested_scope_cleanup
    // Verify that nested scope cleanup doesn't leak
    let (nodes_before, links_before) = count();

    let sig = signal(1);
    let (nodes_after_sig, _) = count();

    // Signal: 1 node
    assert_eq!(nodes_after_sig - nodes_before, 1);

    let scope = scope(move || {
        let _v = sig.get();

        // Create inner scope with signals and computed
        let inner = scope(|| {
            let _s = signal(10);
            let _c = memo(|| 20);
        });

        // Immediately cleanup inner scope
        inner.dispose();
    });

    let (nodes_after_scope, _) = count();

    // outer scope + signal: 2 nodes (inner scope was cleaned up)
    assert_eq!(nodes_after_scope - nodes_after_sig, 1);

    // scope doesn't auto-rerun, so we can't trigger it with sig.set()
    // But we can verify cleanup works
    scope.dispose();
    let (nodes_final, links_final) = count();

    // After cleanup: should return to initial state (only the original signal)
    assert_eq!(nodes_final, nodes_after_sig);
    assert_eq!(links_final, links_before);
}

#[test]
fn test_scope_run_no_leak() {
    let initial = count();

    let sig = signal(1);
    effect(move || {
        sig.get();

        scope(move || {
            effect(move || {
                sig.get();
            });
        })
        .dispose();
    });
    let prev = count();
    sig.set(2);
    sig.set(3);

    assert_eq!(prev, (initial.0 + 2, initial.1 + 1)); // (sig + outter), (sig -> outter)
    assert_eq!(prev, count());
}

#[test]
fn test_effect_run_no_leak() {
    let initial = count();

    let s = signal(1);

    let _e0 = effect(move || {
        let c = computed(move || {
            let _e1 = effect(move || {});
            s.get()
        });
        s.get();

        let _e2 = effect(move || {
            c.get();
        });

        let _e3 = effect(move || {
            s.get();
        });

        _e3.dispose();
    });

    let prev = count();
    s.set(2);
    s.set(3);
    s.set(4);

    // Nodes: initial(root) + s + c + e0 + e1 + e2 + e3 (e3 cleanup removes node)
    // Links: s->e0, e0->e2 (parent subscribes to child), c->e1 (parent subscribes to child),
    //        c->s, e2->c
    assert_eq!(prev, (initial.0 + 5, initial.1 + 5));
    assert_eq!(prev, count());
}
