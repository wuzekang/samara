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

// 回归测试:trigger 订阅多个依赖时,临时节点及其所有链接都应完全回收。
// 复现方式 1:用 count() 确定性观测 —— 多依赖时泄漏恰好一个链接。
#[test]
fn test_trigger_with_multiple_sources_releases_all_links() {
    let src1 = signal(1i32);
    let src2 = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));
    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        *triggers_for_closure.borrow_mut() += 1;
        src1.get();
        src2.get();
    });

    let (nodes_before, links_before) = count();
    trigger(move || {
        src1.get();
        src2.get();
    });
    let (nodes_after, links_after) = count();

    // 临时订阅节点已删除,它建立的链接也应全部解链
    assert_eq!(nodes_after, nodes_before);
    assert_eq!(links_after, links_before);
}

// 复现方式 2:泄漏的链接仍挂在 src2.subs 上且指向已删除的节点,
// trigger 之后再写 src2 会遍历到它(debug 下 slotmap 断言 panic,release 下为 UB)。
#[test]
fn test_trigger_with_multiple_sources_then_set_second_dep() {
    let src1 = signal(1i32);
    let src2 = signal(1i32);

    let triggers = Rc::new(RefCell::new(0i32));
    let triggers_for_closure = triggers.clone();
    let _effect = effect(move || {
        *triggers_for_closure.borrow_mut() += 1;
        src1.get();
        src2.get();
    });

    trigger(move || {
        src1.get();
        src2.get();
    });
    assert_eq!(*triggers.borrow(), 2);

    src2.set(2); // 走到悬挂链接:src2 -> 已删除的临时节点
    assert_eq!(*triggers.borrow(), 3);
}
