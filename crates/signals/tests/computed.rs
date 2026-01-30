use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_computed_basic() {
    let s = signal(10i32);
    let c = memo(move || s.get() * 2);

    assert_eq!(c.get(), 20);

    s.set(20);
    assert_eq!(c.get(), 40);
}

#[test]
fn test_computed_read() {
    let s = signal(10i32);

    #[derive(PartialEq, Clone)]
    struct S {
        value: i32,
    }
    let c = memo(move || S { value: s.get() * 2 });

    assert_eq!(c.read().value, 20);

    s.set(20);
    assert_eq!(c.read().value, 40);
}

#[test]
fn test_computed_caches_result() {
    let s = signal(1i32);
    let computations = Rc::new(RefCell::new(0i32));

    let computations_for_closure = computations.clone();
    let c = memo(move || {
        *computations_for_closure.borrow_mut() += 1;
        s.get() * 2
    });

    assert_eq!(*computations.borrow(), 0);

    c.get();
    assert_eq!(*computations.borrow(), 1);

    c.get();
    assert_eq!(*computations.borrow(), 1); // Should use cache

    s.set(2);
    c.get();
    assert_eq!(*computations.borrow(), 2); // Should recompute
}

#[test]
fn test_computed_chained() {
    let src = signal(0i32);
    let a = memo(move || src.get());
    let b = memo(move || a.get() % 2);
    let c = memo(move || src.get());
    let d = memo(move || b.get() + c.get());

    assert_eq!(d.get(), 0);
    src.set(2);
    assert_eq!(d.get(), 2);
}

#[test]
fn test_computed_propagates_changes() {
    let src = signal(0i32);
    let c1 = memo(move || src.get() % 2);
    let c2 = memo(move || c1.get());
    let c3 = memo(move || c2.get());

    c3.get();
    src.set(1);
    c2.get();
    src.set(3);

    assert_eq!(c3.get(), 1);
}

#[test]
fn test_computed_with_effect() {
    let s = signal(5i32);
    let c = memo(move || s.get() + 1);
    let value = Rc::new(RefCell::new(0i32));

    let value_for_closure = value.clone();
    let _effect = effect(move || {
        *value_for_closure.borrow_mut() = c.get();
    });

    assert_eq!(*value.borrow(), 6);

    s.set(10);
    assert_eq!(*value.borrow(), 11);
}

#[test]
fn test_computed_indirect_updates() {
    let a = signal(false);
    let b = memo(move || a.get());
    let c = memo(move || {
        b.get();
        0
    });
    let d = memo(move || {
        c.get();
        b.get()
    });

    assert_eq!(d.get(), false);
    a.set(true);
    assert_eq!(d.get(), true);
}

#[test]
fn test_multiple_computed_from_same_signal() {
    let src = signal(10i32);
    let c1 = memo(move || src.get() * 2);
    let c2 = memo(move || src.get() + 5);
    let c3 = memo(move || src.get() - 3);

    assert_eq!(c1.get(), 20);
    assert_eq!(c2.get(), 15);
    assert_eq!(c3.get(), 7);

    src.set(20);

    assert_eq!(c1.get(), 40);
    assert_eq!(c2.get(), 25);
    assert_eq!(c3.get(), 17);
}

#[test]
fn test_diamond_dependency() {
    let src = signal(1i32);
    let left = memo(move || src.get() + 1);
    let right = memo(move || src.get() * 2);
    let diamond = memo(move || left.get() + right.get());

    assert_eq!(diamond.get(), 4); // (1 + 1) + (1 * 2) = 4

    src.set(2);
    assert_eq!(diamond.get(), 7); // (2 + 1) + (2 * 2) = 7
}

#[test]
fn test_computed_propagate_chained() {
    let src = signal(0i32);
    let a = memo(move || src.get());
    let b = memo(move || a.get() % 2);
    let c = memo(move || src.get());
    let d = memo(move || b.get() + c.get());

    assert_eq!(d.get(), 0);
    src.set(2);
    assert_eq!(d.get(), 2);
}

#[test]
fn test_computed_not_update_if_reverted() {
    let times = Rc::new(RefCell::new(0i32));
    let src = signal(0i32);

    let times_for_closure = times.clone();
    let c1 = memo(move || {
        *times_for_closure.borrow_mut() += 1;
        src.get()
    });

    c1.get();
    assert_eq!(*times.borrow(), 1);

    src.set(1);
    src.set(0);
    c1.get();
    assert_eq!(*times.borrow(), 2);
}

#[test]
fn test_memo_diamond_effect() {
    let src = signal(1i32);
    let left = memo(move || src.get() + 1);
    let right = memo(move || src.get() * 2);
    let diamond = memo(move || left.get() + right.get());

    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            *c.borrow_mut() += 1;
            diamond.read();
        }
    });

    assert_eq!(diamond.get(), 4);

    src.set(2);
    assert_eq!(diamond.get(), 7);
    src.set(2);

    assert_eq!(*c.borrow(), 2);
}

#[test]
fn test_computed_diamond_effect() {
    let src = signal(1i32);
    let left = computed(move |_| src.get() + 1);
    let right = computed(move |_| src.get() * 2);
    let diamond = computed(move |_| left.get() + right.get());

    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            *c.borrow_mut() += 1;
            diamond.read();
        }
    });

    assert_eq!(diamond.get(), 4);

    src.set(2);
    assert_eq!(*c.borrow(), 2);
    assert_eq!(diamond.get(), 7);

    src.set(2);
    assert_eq!(*c.borrow(), 3);
}
