use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_signal_basic() {
    let s = signal(42i32);
    assert_eq!(s.get(), 42);

    s.set(100);
    assert_eq!(s.get(), 100);
}

#[test]
fn test_signal_bool() {
    let s = signal(true);
    assert_eq!(s.get(), true);

    s.set(false);
    assert_eq!(s.get(), false);
}

#[test]
fn test_signal_read() {
    struct S(Vec<i32>);
    let v = signal(S(vec![1]));
    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            *c.borrow_mut() += 1;
            assert_eq!(v.read().0[0], *c.borrow())
        }
    });

    v.set(S(vec![2]));
    v.set(S(vec![3]));
    v.set(S(vec![4]));
}

#[test]
fn test_signal_peek() {
    struct S(Vec<i32>);
    let v = signal(S(vec![1]));
    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            *c.borrow_mut() += 1;
            assert_eq!(v.peek().0[0], *c.borrow())
        }
    });

    v.set(S(vec![2]));
    v.set(S(vec![3]));

    assert_eq!(*c.borrow(), 1);
}

#[test]
fn test_signal_write() {
    let v = signal(vec![0]);
    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            println!("test_signal_write::effect");
            *c.borrow_mut() += 1;
            assert_eq!(v.read().len(), *c.borrow())
        }
    });

    v.write().push(1);
    v.write().push(2);
    v.write().push(3);

    assert_eq!(*c.borrow(), 4);
}

#[test]
fn test_signal_write_dirty() {
    let v = Signal::new(vec![0]);
    let c = Rc::new(RefCell::new(0));
    effect({
        let c = c.clone();
        move || {
            v.read();
            *c.borrow_mut() += 1;
        }
    });

    v.write().push(1);
    v.write().push(2);

    assert_eq!(*c.borrow(), 3);

    v.set(vec![0, 1, 2]);
    v.set(vec![0, 1, 2]);

    assert_eq!(*c.borrow(), 5);

    v.set(vec![0]);

    assert_eq!(*c.borrow(), 6);
}

#[test]
fn test_signal_does_not_notify_if_unchanged() {
    let s = signal(1i32);
    let triggers = Rc::new(RefCell::new(0i32));

    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        *triggers_for_closure.borrow_mut() += 1;
        s.get();
    });

    assert_eq!(*triggers.borrow(), 1);

    s.set(1);
    assert_eq!(*triggers.borrow(), 2);

    s.set(2);
    assert_eq!(*triggers.borrow(), 3);
}

#[test]
fn test_multiple_modifications_same_signal() {
    let signal = signal(0);

    Effect::new(move || {
        signal.update(|v| *v += 1);
    });
}
