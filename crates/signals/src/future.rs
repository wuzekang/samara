use crate::{Signal, effect, end_batch, runtime::executor::Executor, signal, start_batch};
use std::{future::Future, rc::Rc};

thread_local! {
    pub static EXECUTOR: Executor = Executor::new();
}

/// Spawn an async task on the single-threaded executor
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    EXECUTOR.with(|executor| {
        executor.spawn(future);
    });
}

/// Run all pending async tasks
pub async fn join() {
    EXECUTOR.with(|executor| executor.join()).await
}

pub async fn poll() {
    EXECUTOR.with(|executor| executor.poll()).await
}

pub struct Resource<T> {
    pub value: Signal<Option<T>>,
    pub loading: Signal<bool>,
}

pub fn resource<Func, Fut, Output>(func: Func) -> Resource<Output>
where
    Func: Fn() -> Fut + 'static,
    Fut: Future<Output = Output> + 'static,
    Output: 'static,
{
    let func = signal(Rc::new(func));
    let value = signal(None);
    let loading = signal(true);

    effect(move || {
        spawn(async move {
            let output = (func.get())().await;
            start_batch();
            value.set(Some(output));
            loading.set(false);
            end_batch();
        });
    });

    Resource { value, loading }
}
