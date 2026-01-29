use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_trigger_with_no_dependencies() {
    // Should not throw
    trigger(|| {});
}

#[test]
fn test_trigger_with_signal() {
    let src1 = signal(1i32);
    let src2 = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        *triggers_for_closure.borrow_mut() += 1;
        src1.get();
        src2.get();
    });

    assert_eq!(*triggers.borrow(), 1);

    trigger(move || {
        src1.get();
        src2.get();
    });

    assert_eq!(*triggers.borrow(), 2);
}

#[test]
fn test_trigger_with_multiple_sources() {
    let src1 = signal(1i32);
    let src2 = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        *triggers_for_closure.borrow_mut() += 1;
        src1.get();
        src2.get();
    });

    assert_eq!(*triggers.borrow(), 1);

    trigger(move || {
        src1.get();
        src2.get();
    });

    assert_eq!(*triggers.borrow(), 2);
}
