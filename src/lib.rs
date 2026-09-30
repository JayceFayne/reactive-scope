#![doc = include_str!("../README.md")]
#![cfg_attr(nightly, feature(fn_traits, unboxed_closures))]
#![warn(clippy::all, clippy::pedantic)]

use std::any::Any;

mod graph;
mod set;
#[cfg(test)]
mod tests;

use graph::tls::graph;

pub use graph::{RawSignal, ReadSignal, Scope, Signal, SignalGuard, Snapshot};

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn provide_context<T: Any>(val: T) {
    graph().provide_context(val);
}

#[inline]
#[must_use]
#[cfg_attr(debug_assertions, track_caller)]
pub fn try_use_context<T: Any + Clone>() -> Option<T> {
    graph().try_use_context()
}

#[inline]
#[must_use]
#[cfg_attr(debug_assertions, track_caller)]
pub fn use_context<T: Any + Clone>() -> T {
    graph().use_context()
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_signal<T: Any>(val: T) -> Signal<T> {
    graph().create_signal(val)
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_effect<T, F: FnMut() -> T + 'static>(fun: F) {
    { graph().create_effect(fun) }.run();
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn untrack<T, F: FnOnce() -> T>(fun: F) -> T {
    { graph().untrack() }.run_in(fun)
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn notrack<T, F: FnOnce() -> T>(fun: F) -> T {
    { graph().notrack() }.run_in(fun)
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn nocreate<T, F: FnOnce() -> T>(fun: F) -> T {
    { graph().nocreate() }.run_in(fun)
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn silent<T, F: FnOnce() -> T>(fun: F) -> T {
    { graph().silent() }.run_in(fun)
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_memo<T: 'static, F: FnMut() -> T + 'static>(fun: F) -> ReadSignal<T> {
    let (runner, signal) = graph().create_memo(fun);
    runner.run();
    signal
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_selector<T: 'static + Eq, F: FnMut() -> T + 'static>(fun: F) -> ReadSignal<T> {
    let (runner, signal) = graph().create_selector(fun);
    runner.run();
    signal
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn create_scope() -> Scope {
    graph().create_scope()
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn on_cleanup<T, F: FnOnce() -> T + 'static>(fun: F) {
    graph().on_cleanup(fun);
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn flush() {
    while let Some(effect_key) = { graph().pop_pending_effect() } {
        { graph().effect_runner(effect_key) }.run();
    }
}

#[inline]
pub async fn run() -> ! {
    use crate::flush;
    use std::convert::Infallible;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pub struct RunFuture;

    impl Future for RunFuture {
        type Output = Infallible;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            flush();
            graph().waker(cx.waker().clone());
            Poll::Pending
        }
    }

    RunFuture.await;
    unreachable!()
}
