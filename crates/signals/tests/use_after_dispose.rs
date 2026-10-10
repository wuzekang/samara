use samara_signals::*;
use std::{cell::RefCell, rc::Rc};

// 把 Signal 创建在 scope 里再 dispose,拿到逃逸的 handle
fn disposed_signal() -> Signal<i32> {
    let holder: Rc<RefCell<Option<Signal<i32>>>> = Rc::new(RefCell::new(None));
    let h = holder.clone();
    let sc = scope(move || {
        *h.borrow_mut() = Some(signal(5));
    });
    let sig = holder.borrow().unwrap();
    sc.dispose();
    sig
}

fn disposed_computed() -> Computed<i32> {
    let holder: Rc<RefCell<Option<Computed<i32>>>> = Rc::new(RefCell::new(None));
    let h = holder.clone();
    let sc = scope(move || {
        *h.borrow_mut() = Some(memo(|| 1));
    });
    let c = holder.borrow().unwrap();
    sc.dispose();
    c
}

// dispose 之后所有入口的行为应一致:明确 panic,而不是 unchecked 访问(release 下 UB)
#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn get_after_dispose_panics() {
    disposed_signal().get();
}

#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn track_after_dispose_panics() {
    disposed_signal().track();
}

#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn set_after_dispose_panics() {
    disposed_signal().set(1);
}

#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn update_after_dispose_panics() {
    disposed_signal().update(|v| *v += 1);
}

#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn read_guard_after_dispose_panics() {
    let _ = disposed_signal().read();
}

#[test]
#[should_panic(expected = "Signal accessed after cleanup")]
fn peek_guard_after_dispose_panics() {
    let _ = disposed_signal().peek();
}

#[test]
#[should_panic(expected = "Computed accessed after cleanup")]
fn computed_get_after_dispose_panics() {
    disposed_computed().get();
}

#[test]
#[should_panic(expected = "Computed accessed after cleanup")]
fn computed_peek_after_dispose_panics() {
    let c = disposed_computed();
    let g = c.peek();
    let _ = *g;
}

#[test]
#[should_panic(expected = "Computed accessed after cleanup")]
fn computed_track_after_dispose_panics() {
    disposed_computed().track();
}

// 重复 dispose 应当幂等、不 panic
#[test]
fn double_dispose_is_graceful() {
    let sc = scope(|| {
        signal(1);
    });
    sc.dispose();
    sc.dispose();
}

#[test]
fn double_effect_dispose_is_graceful() {
    let e = effect(|| {});
    e.dispose();
    e.dispose();
}

// guard 存活期间 scope 被 dispose:Drop 不应 panic(否则 unwind 时会 abort)
#[test]
fn guard_alive_across_dispose_drops_cleanly() {
    let holder: Rc<RefCell<Option<Signal<i32>>>> = Rc::new(RefCell::new(None));
    let h = holder.clone();
    let sc = scope(move || {
        *h.borrow_mut() = Some(signal(5));
    });
    let sig = holder.borrow().unwrap();
    let g = sig.read();
    sc.dispose();
    drop(g);
}
