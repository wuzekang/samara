use futures_channel::mpsc;
use futures_util::StreamExt;
use futures_util::stream::{AbortHandle, Abortable, Aborted, FuturesUnordered};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::on_cleanup;
use crate::runtime::REACTIVE_SYSTEM;
use crate::types::NodeKey;

pub struct ReactiveFuture {
    pub scope: NodeKey,
    pub active_sub: Option<NodeKey>,
    pub future: Pin<Box<dyn Future<Output = Result<(), Aborted>> + 'static>>,
}

impl ReactiveFuture {
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        let (scope, active_sub) =
            REACTIVE_SYSTEM.with(|ctx| (ctx.current_scope(), ctx.active_sub()));

        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        on_cleanup({
            move || {
                abort_handle.abort();
            }
        });

        Self {
            scope,
            active_sub,
            future: Box::pin(Abortable::new(future, abort_registration)),
        }
    }
}

impl Future for ReactiveFuture {
    type Output = Result<(), Aborted>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Restore reactive context before polling
        let scope = self.scope;
        let active_sub = self.active_sub;

        // Set captured context
        let (prev_scope, prev_sub) = REACTIVE_SYSTEM.with(|ctx| {
            let prev_scope = ctx.current_scope();
            let prev_sub = ctx.active_sub();
            ctx.set_current_scope(scope);
            ctx.set_active_sub(active_sub);
            (prev_scope, prev_sub)
        });

        let output = self.future.as_mut().poll(cx);

        // Restore previous context
        REACTIVE_SYSTEM.with(|ctx| {
            ctx.set_current_scope(prev_scope);
            ctx.restore_acative_sub(prev_sub);
        });

        output
    }
}

pub struct JoinFuture {
    pub stop: bool,
    pub rx: Rc<RefCell<mpsc::UnboundedReceiver<ReactiveFuture>>>,
    pub tasks: Rc<RefCell<FuturesUnordered<ReactiveFuture>>>,
}

impl Future for JoinFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut rx = self.rx.borrow_mut();
        let mut tasks = self.tasks.borrow_mut();
        let mut dirty = true;
        while dirty {
            while tasks.len() > 0
                && let Poll::Ready(_) = tasks.poll_next_unpin(cx)
            {}
            dirty = false;
            while let Poll::Ready(Some(task)) = { rx.poll_next_unpin(cx) } {
                tasks.push(task);
                dirty = true;
            }
        }
        if tasks.is_empty() && self.stop {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct Executor {
    pub tx: mpsc::UnboundedSender<ReactiveFuture>,
    pub rx: Rc<RefCell<mpsc::UnboundedReceiver<ReactiveFuture>>>,
    pub tasks: Rc<RefCell<FuturesUnordered<ReactiveFuture>>>,
}

impl Executor {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded();
        Self {
            tx,
            rx: Rc::new(RefCell::new(rx)),
            tasks: Default::default(),
        }
    }

    /// Spawn a new task with captured reactive context
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        self.tx.unbounded_send(ReactiveFuture::new(future)).unwrap();
    }

    /// Flush pending tasks to the main task list
    pub fn join(&self) -> JoinFuture {
        JoinFuture {
            stop: true,
            rx: self.rx.clone(),
            tasks: self.tasks.clone(),
        }
    }

    pub fn poll(&self) -> JoinFuture {
        JoinFuture {
            stop: false,
            rx: self.rx.clone(),
            tasks: self.tasks.clone(),
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}
