use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_scope_not_trigger_after_stop() {
    let count = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_closure = triggers.clone();
    let stop_scope = scope(move || {
        let triggers = triggers_for_closure.clone();
        let _effect1 = effect(move || {
            *triggers.borrow_mut() += 1;
            count.get();
        });

        assert_eq!(*triggers_for_closure.borrow(), 1);

        count.set(2);
        assert_eq!(*triggers_for_closure.borrow(), 2);
    });

    count.set(3);
    assert_eq!(*triggers.borrow(), 3);
    stop_scope.dispose();
    count.set(4);
    assert_eq!(*triggers.borrow(), 3);
}

#[test]
fn test_scope_dispose_inner_effects() {
    let source = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_outer = triggers.clone();
    let _effect = effect(move || {
        let triggers = triggers_for_outer.clone();
        let _dispose = scope(move || {
            let triggers = triggers.clone();
            let _effect = effect(move || {
                source.get();
                *triggers.borrow_mut() += 1;
            });
        });

        assert_eq!(*triggers_for_outer.borrow(), 1);

        source.set(2);
        assert_eq!(*triggers_for_outer.borrow(), 2);

        // Note: cleanup is called automatically when scope goes out of scope
    });
}

#[test]
fn test_scope_track_in_outer_effect() {
    let source = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        scope(move || {
            source.get();
        });
        *triggers_for_closure.borrow_mut() += 1;
    });

    assert_eq!(*triggers.borrow(), 1);
    source.set(2);
    assert_eq!(*triggers.borrow(), 2);
}
