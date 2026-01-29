use samara_signals::*;

#[test]
fn test_effect_creates_signal_without_scope_no_leak() {
    // 现在即使没有 scope，effect 也会管理其内部创建的节点
    let s = signal(1);
    let count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let count_clone = count.clone();

    let _e = effect(move || {
        let _inner = signal(1); // ✅ 会被 effect 管理和清理
        *count_clone.borrow_mut() += 1;
        s.get();
    });

    assert_eq!(*count.borrow(), 1);
    s.set(2);
    assert_eq!(*count.borrow(), 2); // effect 重新执行
    // 内部的 signal 被清理并重新创建，无泄漏
}

#[test]
fn test_nested_effect_cleanup() {
    // effect 中创建嵌套 effect，旧的 inner effect 会被清理
    let s = signal(1);

    let _outer = effect(move || {
        let _inner = effect(move || {
            s.get();
        });
    });
}

#[test]
fn test_effect_reexecution_cleans_children() {
    // 验证 effect 重新执行时会清理旧的子节点
    let s = signal(1);
    let run_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let run_count_clone = run_count.clone();

    let _e = effect(move || {
        *run_count_clone.borrow_mut() += 1;
        let _temp = signal(100);
        s.get();
    });

    assert_eq!(*run_count.borrow(), 1);
    s.set(2);
    assert_eq!(*run_count.borrow(), 2);
}
