use crate::{Signal, effect, end_batch, runtime::executor::Executor, signal, start_batch};
use std::future::Future;

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

/// A reactive async value: re-fetches whenever signals read inside `func`
/// change, exposing the latest result and a loading flag.
///
/// The closure runs synchronously on every (re)fetch — signal reads inside
/// it are tracked, so they become the resource's dependencies. The returned
/// future is spawned on the local executor; when a dependency changes again,
/// the effect re-runs, and the in-flight future is aborted via its cleanup
/// registration, so a stale result is never written.
///
/// While re-fetching, `value` keeps the previous result and `loading` is
/// `true`. Errors are not modeled — make `Output` a `Result` if needed.
///
/// # Example
///
/// ```
/// # use samara_signals::*;
/// # async fn fetch_user(id: i32) -> String { format!("user{id}") }
/// # async fn demo() {
/// let user_id = signal(1);
/// let r = resource(move || {
///     let id = user_id.get(); // tracked: refetch on change
///     async move { fetch_user(id).await }
/// });
/// join().await;
/// assert_eq!(r.value.get(), Some(String::from("user1")));
/// # }
/// ```
pub fn resource<Func, Fut, Output>(func: Func) -> Resource<Output>
where
    Func: Fn() -> Fut + 'static,
    Fut: Future<Output = Output> + 'static,
    Output: 'static,
{
    let value = signal(None);
    let loading = signal(true);

    effect(move || {
        // Call func HERE (not inside the spawned task) so signal reads are
        // tracked by this effect — that is what makes the resource re-fetch.
        let fut = func();
        loading.set(true);
        spawn(async move {
            let output = fut.await;
            start_batch();
            value.set(Some(output));
            loading.set(false);
            end_batch();
        });
    });

    Resource { value, loading }
}
