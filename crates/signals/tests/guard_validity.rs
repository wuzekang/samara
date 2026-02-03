use samara_signals::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
#[should_panic]
fn test_read_guard_after_cleanup() {
    let s = Rc::new(RefCell::new(None));

    let scope = scope({
        let s = s.clone();
        move || {
            *s.borrow_mut() = Some(signal(42i32));
        }
    });

    let s = s.borrow().unwrap();
    let g = s.read();

    scope.dispose(); // Cleanup scope and its signals

    // Now accessing the guard should panic
    // let _x = *g; // This should panic with "Signal accessed after cleanup"
}

#[test]
#[should_panic]
fn test_write_guard_after_cleanup() {
    let s = Rc::new(RefCell::new(None));

    let scope = scope({
        let s = s.clone();
        move || {
            *s.borrow_mut() = Some(signal(vec![1i32, 2, 3]));
        }
    });

    let s = s.borrow().unwrap();
    let mut g = s.write();

    scope.dispose(); // Cleanup scope and its signals

    // Now accessing the guard should panic
    // g.push(4); // This should panic with "Signal accessed after cleanup"
}

#[test]
fn test_guard_valid_within_scope() {
    scope(|| {
        let s = signal(42i32);
        let guard = s.read();
        assert_eq!(*guard, 42);
        // Still valid - scope not disposed
    });
}

#[test]
fn test_multiple_guards_same_signal() {
    scope(|| {
        let s = signal(42i32);
        let g1 = s.read();
        let g2 = s.read(); // Multiple reads OK
        assert_eq!(*g1, *g2);
    });
}

#[test]
fn test_guard_valid_outlives_scope() {
    let s = Rc::new(RefCell::new(None));

    let s_clone = s.clone();
    let _ = scope(move || {
        let signal = signal(1i32);
        *s_clone.borrow_mut() = Some(signal);
    });

    let signal = s.borrow().unwrap();
    assert_eq!(*signal.read() + 1, 2);
}
