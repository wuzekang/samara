use samara_signals::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Duration;

#[tokio::test]
async fn test_spawn_basic() {
    let sig = signal(0);

    spawn(async move {
        sig.set(42);
    });

    join().await;
    assert_eq!(sig.get(), 42);
}

#[tokio::test]
async fn test_signal_access_from_async() {
    let count = signal(0);
    let result = signal(0);

    count.set(21);

    spawn(async move {
        let c = count.get();
        result.set(c * 2);
    });

    join().await;
    assert_eq!(result.get(), 42);
}

#[tokio::test]
async fn test_context_access_from_async() {
    scope(|| {
        provide_context(42i32);

        spawn(async move {
            let ctx = use_context::<i32>().unwrap();
            assert_eq!(ctx, 42);
        });
    });
    join().await;
}

#[tokio::test]
async fn test_on_cleanup_from_async() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let cleaned_clone = cleaned.clone();

    let s = scope(move || {
        spawn(async move {
            on_cleanup(move || {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        });
    });
    join().await;

    // Manually dispose the scope to trigger cleanup
    s.dispose();

    // Scope disposal should trigger cleanup
    assert!(cleaned.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_effect_with_spawn() {
    let count = signal(0);
    let result = signal(0);

    effect(move || {
        let count = count;
        let result = result;

        spawn(async move {
            let c = count.get();
            result.set(c * 2);
        });
    });

    count.set(21);
    join().await;

    assert_eq!(result.get(), 42);
}

#[tokio::test]
async fn test_multiple_spawn() {
    let sig1 = signal(0);
    let sig2 = signal(0);
    let sig3 = signal(0);

    spawn(async move {
        sig1.set(1);
    });

    spawn(async move {
        sig2.set(2);
    });

    spawn(async move {
        sig3.set(3);
    });

    join().await;
    assert_eq!(sig1.get(), 1);
    assert_eq!(sig2.get(), 2);
    assert_eq!(sig3.get(), 3);
}

#[tokio::test]
async fn test_nested_spawn() {
    let result = signal(0);

    spawn(async move {
        spawn(async move {
            result.set(42);
        });
    });

    join().await;
    assert_eq!(result.get(), 42);
}

#[tokio::test]
async fn test_async_with_computed() {
    let source = signal(10);
    let comp = computed(move || source.get() * 2);
    let result = signal(0);

    spawn(async move {
        let val = comp.get();
        result.set(val);
    });

    join().await;
    assert_eq!(result.get(), 20);
}

#[tokio::test]
async fn test_async_task_cleanup_on_scope_dispose() {
    let cleaned = Arc::new(AtomicBool::new(false));
    let cleaned_clone = cleaned.clone();

    let s = scope(move || {
        spawn(async move {
            on_cleanup(move || {
                cleaned_clone.store(true, Ordering::SeqCst);
            });
        });
    });

    join().await;
    s.dispose();

    assert!(cleaned.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_effect_dispose_with_spawn() {
    let result = Arc::new(AtomicI32::new(0));
    let ran = Arc::new(AtomicBool::new(false));

    let result_clone = result.clone();
    let ran_clone = ran.clone();

    effect(move || {
        let result = result_clone.clone();
        let ran = ran_clone.clone();

        spawn(async move {
            ran.store(true, Ordering::SeqCst);
            result.store(42, Ordering::SeqCst);
        });
    });

    join().await;

    assert_eq!(result.load(Ordering::SeqCst), 42);
    assert!(ran.load(Ordering::SeqCst));

    // Dispose the current scope (which includes the effect)
    cleanup();

    // Reset and trigger again - effect should not run
    ran.store(false, Ordering::SeqCst);

    // Effect should not have run again after disposal
    assert!(!ran.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_spawn_preserves_context_multiple_polls() {
    let count = signal(0);
    let result = signal(0);

    effect(move || {
        let count = count;
        let result = result;

        spawn(async move {
            // First read
            let c1 = count.get();

            // Simulate some async work (await point)
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Second read after await - context should still be preserved
            let c2 = count.get();

            result.set(c1 + c2);
        });
    });

    count.set(10);
    join().await;

    assert_eq!(result.get(), 20);
}

#[tokio::test]
async fn test_multiple_async_tasks_in_effect() {
    let sum = signal(0);
    provide_context(1);
    effect(move || {
        let sum = sum;

        // Spawn multiple tasks
        for i in 1..=5 {
            spawn(async move { *sum.write() += i + use_context::<i32>().unwrap() });
        }
    });

    join().await;

    // Should sum 1+2+3+4+5 = 15
    assert_eq!(sum.get(), 20);
}

#[tokio::test]
async fn test_multiple_async_tasks_in_effect_tokio() {
    let sum = signal(0);
    provide_context(1);
    effect(move || {
        let sum = sum;

        // Spawn multiple tasks
        for i in 1..=5 {
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(i as u64)).await;
                *sum.write() += i + use_context::<i32>().unwrap()
            });
        }
    });

    join().await;

    // Should sum 1+2+3+4+5 = 15
    assert_eq!(sum.get(), 20);
}

#[tokio::test]
async fn test_async_race_condition() {
    let s = signal(vec![]);

    scope(move || {
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            s.write().push(200);
        });

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            s.write().push(100);
        });
    })
    .dispose();

    scope(move || {
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            s.write().push(2);
        });

        spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            s.write().push(1);
        });
    });

    join().await;

    assert_eq!(s.get(), vec![1, 2]);
}

#[tokio::test]
async fn test_async_resource() {
    let s = signal(1);

    let Resource { value, loading } = resource(move || async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        s.get() * 2
    });

    assert_eq!(value.get(), None);
    assert_eq!(loading.get(), true);

    s.set(2);

    assert_eq!(value.get(), None);
    assert_eq!(loading.get(), true);

    join().await;

    assert_eq!(value.get(), Some(4));
    assert_eq!(loading.get(), false);

    s.set(3);
    assert_eq!(value.get(), Some(4));
    assert_eq!(loading.get(), false);

    join().await;
    assert_eq!(value.get(), Some(6));
    assert_eq!(loading.get(), false);
}
