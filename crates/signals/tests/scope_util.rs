use samara_signals::*;

#[test]
fn test_scoped_basic() {
    let scoped_fn = scoped(|x: i32| x + 1);
    let (result, scope) = scoped_fn(5);
    assert_eq!(result, 6);
    scope.dispose();
}

#[test]
fn test_scoped_with_reactive() {
    let _s = signal(0);
    let scoped_fn = scoped(|_| {
        let s = signal(42);
        s.get()
    });
    let (result, scope) = scoped_fn(());
    assert_eq!(result, 42);
    scope.dispose();
}

#[test]
fn test_scoped_cleanup_works() {
    let (nodes_before, _) = count();

    let scoped_fn = scoped(|_| {
        let _s = signal(1);
        let _c = computed(move || 2);
    });
    let (_, scope) = scoped_fn(());

    let (nodes_with_scope, _) = count();
    assert_eq!(nodes_with_scope - nodes_before, 3); // scope + signal + computed

    scope.dispose();

    let (nodes_after, _) = count();
    assert_eq!(nodes_after, nodes_before); // All cleaned up
}

#[test]
fn test_scoped_nested() {
    let outer_fn = scoped(|_| {
        let _s1 = signal(1);

        // Create another child scope inside
        let inner_fn = scoped(|_| {
            let _s2 = signal(2);
        });

        inner_fn(());
    });

    let (_, outer_scope) = outer_fn(());

    // Cleanup outer scope should clean up inner scope too
    outer_scope.dispose();
}

#[test]
fn test_scoped_deferred_execution() {
    // Create the scoped function
    let scoped_fn = scoped(|x: i32| x * 2);

    // Execute it multiple times
    let (result1, scope1) = scoped_fn(5);
    assert_eq!(result1, 10);

    let (result2, scope2) = scoped_fn(10);
    assert_eq!(result2, 20);

    // Cleanup both scopes
    scope1.dispose();
    scope2.dispose();
}

#[test]
fn test_scoped_captures_parent_at_creation() {
    // Create a scope
    let scope = scope(|| {
        // Capture this scope as parent
        let scoped_fn = scoped(|_| {
            // This should be a child of the scope
            let _s = signal(123);
        });

        // Execute the scoped function
        scoped_fn(());
    });

    // Cleanup should remove all children
    scope.dispose();
}

#[test]
fn test_scoped_with_effect() {
    let scoped_fn = scoped(|_| {
        let counter = signal(0);
        let _effect = effect(move || {
            counter.set(counter.get() + 1);
        });
    });

    let (_, scope) = scoped_fn(());
    scope.dispose();
}

#[test]
fn test_scoped_multiple_calls() {
    let scoped_fn = scoped(|x: i32| x + 1);

    // Call multiple times before cleanup
    let (r1, s1) = scoped_fn(1);
    let (r2, s2) = scoped_fn(2);
    let (r3, s3) = scoped_fn(3);

    assert_eq!(r1, 2);
    assert_eq!(r2, 3);
    assert_eq!(r3, 4);

    // Cleanup all scopes
    s1.dispose();
    s2.dispose();
    s3.dispose();
}
