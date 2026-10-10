use samara_signals::*;
use std::{cell::RefCell, rc::Rc};

// signal_update 的回调运行期间系统借用必须已释放:
// 回调里可以自由重入响应式系统(写其他信号、触发 effect)。
#[test]
fn update_callback_may_reenter() {
    let a = signal(1i32);
    let b = signal(2i32);

    let runs = Rc::new(RefCell::new(0));
    let r = runs.clone();
    let _e = effect(move || {
        *r.borrow_mut() += 1;
        b.get();
    });

    a.update(|v| {
        *v += 1;
        b.set(10); // 重入:set → notify → flush → effect
    });

    assert_eq!(a.get_untracked(), 2);
    assert_eq!(b.get_untracked(), 10);
    assert_eq!(*runs.borrow(), 2);
}

// 回调里把自己的 scope dispose 掉:值被安全释放,不 panic、不 notify 死节点。
#[test]
fn update_callback_may_dispose_owning_scope() {
    let holder: Rc<RefCell<Option<Signal<i32>>>> = Rc::new(RefCell::new(None));
    let h = holder.clone();
    let sc = scope(move || {
        *h.borrow_mut() = Some(signal(5));
    });
    let sig = holder.borrow().unwrap();

    sig.update(|v| {
        *v += 1;
        sc.dispose(); // 在回调里销毁信号所属的 scope
    });
}

// getter 运行期间把自己的节点销毁(cleanup 会移除 root 的全部子节点):
// 取值-回调-回填模式下,getter 正常跑完,存储 Box 自然释放,之后访问
// 是干净的 panic 而不是对已删槽位的未检查访问。
#[test]
#[should_panic(expected = "Computed accessed after cleanup")]
fn getter_disposing_own_node_is_handled() {
    let c = computed(|prev: Option<i32>| {
        cleanup(); // 移除 root 子节点,包括这个 computed 自己
        prev.unwrap_or(0) + 1
    });
    let _ = c.get();
}

// effect 重跑时,函数体内销毁自己:闭包跑完、正常返回,不 panic 不 UB
#[test]
fn effect_body_disposing_itself_completes() {
    let s = signal(0);
    let completed = Rc::new(std::cell::Cell::new(false));
    let c2 = completed.clone();
    let _e = effect(move || {
        if s.get() == 1 {
            cleanup(); // 在 run() 的回调中移除自己(以及 s)
            c2.set(true);
        }
    });
    s.set(1); // 触发 run():体内 cleanup 后闭包仍执行完毕
    assert!(completed.get());
}

// computed 循环依赖(getter 读自己):必须 panic,不能静默返回错值
#[test]
#[should_panic(expected = "Node is not a Computed")]
fn computed_cycle_panics() {
    let holder: Rc<RefCell<Option<Computed<i32>>>> = Rc::new(RefCell::new(None));
    let h = holder.clone();
    let h2 = holder.clone();
    let _sc = scope(move || {
        *h.borrow_mut() = Some(computed(move |prev: Option<i32>| {
            h2.borrow().unwrap().get(); // 读自己 → 循环
            prev.unwrap_or(0) + 1
        }));
    });
    let _ = holder.borrow().unwrap().get();
}
