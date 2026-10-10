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
