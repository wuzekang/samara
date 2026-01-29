use samara_signals::*;

#[test]
fn test_topology_drop_a_b_a_updates() {
    //     A
    //   / |
    //  B  | <- Looks like a flag doesn't it? :D
    //   \ |
    //     C
    //     |
    //     D
    let a = signal(2i32);

    let b = memo(move || a.get() - 1);
    let c = memo(move || a.get() + b.get());

    let compute_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let compute_count_for_closure = compute_count.clone();
    let d = memo(move || {
        *compute_count_for_closure.borrow_mut() += 1;
        format!("d: {}", c.get())
    });

    // Trigger read
    assert_eq!(d.get(), "d: 3");
    assert_eq!(*compute_count.borrow(), 1);

    a.set(4);
    d.get();
    assert_eq!(*compute_count.borrow(), 2);
}

#[test]
fn test_topology_diamond_graph() {
    // In this scenario "D" should only update once when "A" receives
    // an update. This is sometimes referred to as the "diamond" scenario.
    //     A
    //   /   \
    //  B     C
    //   \   /
    //     D

    let a = signal("a");
    let b = memo(move || a.get().to_string());
    let c = memo(move || a.get().to_string());

    let d_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let d_count_for_closure = d_count.clone();
    let d = memo(move || {
        *d_count_for_closure.borrow_mut() += 1;
        format!("{} {}", b.get(), c.get())
    });

    // First access triggers computation
    assert_eq!(d.get(), "a a");
    assert_eq!(*d_count.borrow(), 1);

    a.set("aa");
    // Second access after update
    assert_eq!(d.get(), "aa aa");
    assert_eq!(*d_count.borrow(), 2);
}

#[test]
fn test_topology_diamond_with_tail() {
    // "E" will be likely updated twice if our mark+sweep logic is buggy.
    //     A
    //   /   \
    //  B     C
    //   \   /
    //     D
    //     |
    //     E

    let a = signal("a");
    let b = memo(move || a.get().to_string());
    let c = memo(move || a.get().to_string());

    let d = memo(move || format!("{} {}", b.get(), c.get()));

    let e_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let e_count_for_closure = e_count.clone();
    let e = memo(move || {
        *e_count_for_closure.borrow_mut() += 1;
        d.get()
    });

    // First access triggers computation
    assert_eq!(e.get(), "a a");
    assert_eq!(*e_count.borrow(), 1);

    a.set("aa");
    // Second access after update
    assert_eq!(e.get(), "aa aa");
    assert_eq!(*e_count.borrow(), 2);
}

#[test]
fn test_topology_bail_out_if_same() {
    // Bail out if value of "B" never changes
    // A->B->C
    let a = signal("a");
    let b = memo(move || {
        a.get();
        "foo"
    });

    let c_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let c_count_for_closure = c_count.clone();
    let c = memo(move || {
        *c_count_for_closure.borrow_mut() += 1;
        b.get()
    });

    // First access triggers computation
    assert_eq!(c.get(), "foo");
    assert_eq!(*c_count.borrow(), 1);

    a.set("aa");
    // Second access - B returns same value, so C should not recompute
    assert_eq!(c.get(), "foo");
    assert_eq!(*c_count.borrow(), 1);
}

#[test]
fn test_topology_jagged_diamond_with_tails() {
    // "F" and "G" will be likely updated twice if our mark+sweep logic is buggy.
    //     A
    //   /   \
    //  B     C
    //  |     |
    //  |     D
    //   \   /
    //     E
    //   /   \
    //  F     G
    let a = signal("a");

    let b = memo(move || a.get().to_string());
    let c = memo(move || a.get().to_string());

    let d = memo(move || c.get().to_string());

    let e_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let e_count_for_closure = e_count.clone();
    let e = memo(move || {
        *e_count_for_closure.borrow_mut() += 1;
        format!("{} {}", b.get(), d.get())
    });

    // First access triggers computation
    assert_eq!(e.get(), "a a");
    assert_eq!(*e_count.borrow(), 1);

    a.set("b");
    // Second access after update
    assert_eq!(e.get(), "b b");
    assert_eq!(*e_count.borrow(), 2);
}

#[test]
fn test_topology_ensure_subs_update_even_if_one_dep_unmarks() {
    // In this scenario "C" always returns the same value. When "A"
    // changes, "B" will update, then "C" at which point its update
    // to "D" will be unmarked. But "D" must still update because
    // "B" marked it. If "D" isn't updated, then we have a bug.
    //     A
    //   /   \
    //  B     *C <- returns same value every time
    //   \   /
    //     D
    let a = signal("a");
    let b = memo(move || a.get());
    let c = memo(move || {
        a.get();
        "c"
    });

    let d_value = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let d = memo(move || format!("{} {}", b.get(), c.get()));

    *d_value.borrow_mut() = d.get();
    assert_eq!(*d_value.borrow(), "a c");

    a.set("aa");
    *d_value.borrow_mut() = d.get();
    assert_eq!(*d_value.borrow(), "aa c");
}

#[test]
fn test_topology_ensure_subs_update_even_if_two_deps_unmark() {
    // In this scenario both "C" and "D" always return the same
    // value. But "E" must still update because "A" marked it.
    // If "E" isn't updated, then we have a bug.
    //     A
    //   / | \
    //  B *C *D
    //   \ | /
    //     E
    let a = signal("a");
    let b = memo(move || a.get());
    let c = memo(move || {
        a.get();
        "c"
    });
    let d = memo(move || {
        a.get();
        "d"
    });

    let e_value = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let e = memo(move || format!("{} {} {}", b.get(), c.get(), d.get()));

    *e_value.borrow_mut() = e.get();
    assert_eq!(*e_value.borrow(), "a c d");

    a.set("aa");
    *e_value.borrow_mut() = e.get();
    assert_eq!(*e_value.borrow(), "aa c d");
}

#[test]
fn test_topology_support_lazy_branches() {
    let a = signal(0i32);
    let b = memo(move || a.get());
    let c = memo(move || if a.get() > 0 { a.get() } else { b.get() });

    assert_eq!(c.get(), 0);
    a.set(1);
    assert_eq!(c.get(), 1);

    a.set(0);
    assert_eq!(c.get(), 0);
}

#[test]
fn test_topology_not_update_sub_if_all_deps_unmark() {
    // In this scenario "B" and "C" always return the same value. When "A"
    // changes, "D" should not update.
    //     A
    //   /   \
    // *B     *C
    //   \   /
    //     D
    let a = signal("a");
    let b = memo(move || {
        a.get();
        "b"
    });
    let c = memo(move || {
        a.get();
        "c"
    });

    let d_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let d_count_for_closure = d_count.clone();
    let d = memo(move || {
        *d_count_for_closure.borrow_mut() += 1;
        format!("{} {}", b.get(), c.get())
    });

    assert_eq!(*d_count.borrow(), 0);

    d.get();
    assert_eq!(*d_count.borrow(), 1);

    a.set("aa");
    assert_eq!(*d_count.borrow(), 1);
}
