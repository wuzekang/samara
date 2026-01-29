use std::{cell::RefCell, rc::Rc};

use samara_signals::*;

#[test]
fn test_nested_effect_with_cleanup() {
    let sig = signal(1);
    let run_count = Rc::new(RefCell::new(0));

    effect({
        let run_count = run_count.clone();
        move || {
            let _v = sig.get();
            *run_count.borrow_mut() += 1;

            // Inner effect
            let inner = effect(move || {
                // inner effect doesn't subscribe to anything
            });

            // Cleanup inner effect
            inner.dispose();
        }
    });

    // Trigger outer effect to run again
    sig.set(2);

    // This test used to panic with "invalid SlotMap key used"
    // because after the inner effect was deleted, the outer effect
    // would try to access already-deleted nodes when running again
    assert_eq!(*run_count.borrow(), 2);
}

#[test]
fn test_effect_creates_and_destroys_inner_effect() {
    let sig = signal(1);
    let inner_run_count = Rc::new(RefCell::new(0));
    let outer_run_count = Rc::new(RefCell::new(0));

    effect({
        let inner_run_count = inner_run_count.clone();
        let outer_run_count = outer_run_count.clone();
        move || {
            let _v = sig.get();
            *outer_run_count.borrow_mut() += 1;

            // Create inner effect
            let inner = effect({
                let inner_run_count = inner_run_count.clone();
                move || {
                    *inner_run_count.borrow_mut() += 1;
                }
            });

            // Immediately cleanup
            inner.dispose();
        }
    });

    // First run
    sig.set(2);

    // Inner effect should be created 2 times, but cleaned up each time
    assert_eq!(*inner_run_count.borrow(), 2);
    assert_eq!(*outer_run_count.borrow(), 2);
}

#[test]
fn test_nested_scope_cleanup() {
    let sig = signal(1);
    let outer_runs = Rc::new(RefCell::new(0));

    // scope only runs once, doesn't automatically re-execute
    // so outer_runs expected value is 1, not 2
    let _scope = scope({
        let outer_runs = outer_runs.clone();
        move || {
            let _v = sig.get();
            *outer_runs.borrow_mut() += 1;

            // Inner scope
            let inner_scope = scope(|| {
                // Create some nodes
                let _s = signal(10);
                let _c = memo(|| 20);
            });

            // Cleanup inner scope
            inner_scope.dispose();
        }
    });

    // scope won't re-run due to sig.set()
    sig.set(2);

    // Verify it only ran once
    assert_eq!(*outer_runs.borrow(), 1);
}
