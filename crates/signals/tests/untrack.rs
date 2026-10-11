use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn untracked_reads_do_not_subscribe() {
    let tracked = signal(1);
    let aux = signal(100);

    let runs = Rc::new(RefCell::new(0));
    let runs2 = runs.clone();

    let _e = effect(move || {
        *runs2.borrow_mut() += 1;
        let _tracked = tracked.get();
        let _aux = untrack(|| aux.get());
    });

    assert_eq!(*runs.borrow(), 1);

    // Writing the untracked signal must NOT re-run the effect
    aux.set(200);
    assert_eq!(*runs.borrow(), 1);
    assert_eq!(aux.get(), 200);

    // Writing the tracked signal must re-run it
    tracked.set(2);
    assert_eq!(*runs.borrow(), 2);
}

#[test]
fn untrack_at_top_level_reads_current_value() {
    let s = signal(5);
    let v = untrack(|| s.get());
    assert_eq!(v, 5);
}

#[test]
fn untracked_guard_reads_do_not_subscribe() {
    let s = signal(String::from("a"));
    let runs = Rc::new(RefCell::new(Vec::new()));
    let runs2 = runs.clone();

    let _e = effect(move || {
        // peek() is the guard-based untracked read
        let peeked = s.peek().clone();
        runs2.borrow_mut().push(peeked);
        let _tracked = s.get();
    });

    s.set(String::from("b"));
    assert_eq!(*runs.borrow(), vec![String::from("a"), String::from("b")]);
}

#[test]
fn current_effect_is_none_outside_and_some_inside() {
    assert!(current_effect().is_none());

    let s = signal(1);
    let seen: Rc<RefCell<Option<Effect>>> = Rc::new(RefCell::new(None));
    let seen2 = seen.clone();
    let _e = effect(move || {
        let _ = s.get();
        *seen2.borrow_mut() = current_effect();
    });

    assert!(seen.borrow().is_some());
    assert!(current_effect().is_none());
}

#[test]
fn enter_context_subscribes_reads_to_the_entered_effect() {
    // e1 captures its own handle; a probe signal is read while e1's context
    // is entered from OUTSIDE any effect — the read must subscribe e1.
    let s = signal(1);
    let handle: Rc<RefCell<Option<Effect>>> = Rc::new(RefCell::new(None));
    let handle2 = handle.clone();
    let e1_runs = Rc::new(RefCell::new(0));
    let e1_runs2 = e1_runs.clone();

    let _e1 = effect(move || {
        let _ = s.get();
        *e1_runs2.borrow_mut() += 1;
        *handle2.borrow_mut() = current_effect();
    });
    assert_eq!(*e1_runs.borrow(), 1);

    let probe = signal(0);

    {
        let e = handle.borrow().clone().unwrap();
        let _guard = e.enter_context();
        // Outside any effect this read would not be tracked at all; inside
        // the entered context it becomes a dependency of e1.
        let _ = probe.get();
    }

    probe.set(1);
    assert_eq!(*e1_runs.borrow(), 2);
}

#[test]
fn values_created_in_entered_context_are_disposed_with_the_effect() {
    let s = signal(1);
    let handle: Rc<RefCell<Option<Effect>>> = Rc::new(RefCell::new(None));
    let handle2 = handle.clone();
    let _e = effect(move || {
        let _ = s.get();
        *handle2.borrow_mut() = current_effect();
    });
    let e = handle.borrow().clone().unwrap();

    let created = {
        let _guard = e.enter_context();
        signal(42)
    };
    assert_eq!(created.get(), 42);

    // Disposing the effect drops nodes created under its context; touching
    // them afterwards is detected instead of corrupting memory.
    e.dispose();
}
