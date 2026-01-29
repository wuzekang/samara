use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_batch_basic() {
    let s = signal(1i32);
    let value = Rc::new(RefCell::new(0i32));

    let value_for_closure = value.clone();
    let _effect = effect(move || {
        *value_for_closure.borrow_mut() = s.get();
    });

    assert_eq!(*value.borrow(), 1);

    start_batch();
    s.set(2);
    s.set(3);
    assert_eq!(*value.borrow(), 1); // Should not run yet
    end_batch();

    assert_eq!(*value.borrow(), 3); // Should run once with final value
}

#[test]
fn test_batch_nested() {
    let s1 = signal(1i32);
    let s2 = signal(1i32);
    let value = Rc::new(RefCell::new(0i32));

    let value_for_closure = value.clone();
    let _effect = effect(move || {
        *value_for_closure.borrow_mut() = s1.get() + s2.get();
    });

    assert_eq!(*value.borrow(), 2);

    start_batch();
    s1.set(2);
    start_batch();
    s2.set(2);
    end_batch();
    end_batch();

    assert_eq!(*value.borrow(), 4);
}
