use std::{cell::RefCell, rc::Rc};

use samara_signals::*;

#[test]
fn test_scope_cleanup_removes_nodes() {
    let scope = scope(|| {
        let _s = signal(1);
        let _c = memo(|| {
            let s = signal(1);
            s.get() * 2
        });
        let _e = effect(|| {
            let s = signal(1);
            s.get();
        });
    });
    scope.dispose();
}

#[test]
fn test_nested_scope_cleanup() {
    let outer = scope(|| {
        let _s = signal(1);
        let _inner = scope(|| {
            let s = signal(1);
            let _c = memo(move || s.get() + 1);
        });
    });
    outer.dispose();
}

#[test]
fn test_links_removed_on_cleanup() {
    let vec = Rc::new(RefCell::new(Vec::<i32>::new()));
    let scope = scope({
        let vec = vec.clone();
        move || {
            let s = signal(1);
            let _c = memo(move || s.get() + 1);
            on_cleanup({
                let vec = vec.clone();
                move || {
                    vec.borrow_mut().push(0);
                }
            });
            on_cleanup({
                let vec = vec.clone();
                move || {
                    vec.borrow_mut().push(1);
                }
            });
            on_cleanup({
                let vec = vec.clone();
                move || {
                    vec.borrow_mut().push(2);
                }
            });
        }
    });

    assert_eq!(*vec.borrow(), Vec::<i32>::new());
    scope.dispose();
    assert_eq!(*vec.borrow(), vec![2, 1, 0]);
}

#[test]
fn test_scope_with_signal_cleanup() {
    let scope = scope(|| {
        let s = signal(1);
        let c = memo(move || s.get() * 2);
        let _v = c.get();
        s.set(2);
        let _v2 = c.get();
    });
    scope.dispose();
}

#[test]
fn test_cleanup_is_idempotent() {
    let scope = scope(|| {
        let _s = signal(1);
        let _e = effect(|| {
            let s = signal(1);
            s.get();
        });
    });
    scope.dispose();
    scope.dispose();
}

#[test]
fn test_nested_effect_with_computed() {
    let s = signal(1);

    effect(move || {
        let c = computed(move |_| s.get());
        c.get();

        effect(move || {}).dispose();
        effect(move || {}).dispose();
    });

    s.set(2);
    s.set(3);
}

#[test]
fn test_signal_access_in_cleanup() {
    let s = signal(2);
    let cnt = signal(0);
    let scope = scope(move || {
        effect(move || {
            let c = computed(move |_| s.get() * 2);
            c.get();

            on_cleanup(move || {
                *cnt.write() += 1;
                let t = signal(0);
                t.read();
                assert_eq!(c.get(), s.get() * 2)
            });
        });
    });

    s.set(2);
    s.set(3);
    s.set(4);

    assert_eq!(cnt.get(), 3);

    scope.dispose();

    assert_eq!(cnt.get(), 4);
}

#[test]
fn test_nest_cleanup() {
    let vec = signal(vec![]);
    scope({
        move || {
            let s = signal(1);
            let _c = memo(move || s.get() + 1);
            on_cleanup(move || vec.write().push(0));
            on_cleanup(move || {
                vec.write().push(1);
            });
            scope(move || {
                on_cleanup(move || {
                    vec.write().push(11);
                });
                on_cleanup(move || {
                    vec.write().push(12);
                });
            });
            on_cleanup(move || {
                vec.write().push(2);
            });
            scope(move || {
                on_cleanup(move || {
                    vec.write().push(21);
                });
                on_cleanup(move || {
                    vec.write().push(22);
                });

                effect(move || {
                    on_cleanup(move || {
                        vec.write().push(221);
                    });
                    on_cleanup(move || {
                        vec.write().push(222);
                    });
                });
            });
            on_cleanup(move || {
                vec.write().push(3);
            });
        }
    })
    .dispose();

    assert_eq!(vec.get(), vec![222, 221, 22, 21, 12, 11, 3, 2, 1, 0]);
}
