use super::tls::{graph, try_graph};
use super::{Graph, ScopeKey};
use std::fmt::Debug;
use std::marker::PhantomData;

#[derive(Debug)]
#[must_use = "scope owns reactive signals and effects"]
pub struct Scope {
    key: ScopeKey,
    marker: PhantomData<*mut ()>,
}

impl Scope {
    #[inline]
    pub(crate) const fn new(key: ScopeKey) -> Scope {
        Scope {
            key,
            marker: PhantomData,
        }
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn run_in<O, F: FnOnce() -> O>(&self, fun: F) -> O {
        { graph().scope(self.key) }.run_in(fun)
    }

    #[inline]
    #[must_use]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn is_root(&self) -> bool {
        graph()[self.key].parent.is_none()
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn detach(&self) {
        graph().detach(self.key);
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn attach(&self) {
        graph().attach(self.key);
    }
}

fn drop_scope(graph: &mut Graph, scope_key: ScopeKey) {
    graph.batching.scope_stack.push(scope_key);
    let mut pos = 0;
    while let Some(&scope_key) = graph.batching.scope_stack.get(pos) {
        if let Some(scope) = graph.scopes.get(scope_key) {
            for &scope_key in scope.children.iter() {
                graph.batching.scope_stack.push(scope_key);
            }
        }
        pos += 1;
    }
    for scope_key in graph.batching.scope_stack.drain(..).rev() {
        if let Some(scope) = graph.scopes.remove(scope_key) {
            for cleanup in scope.cleanup.into_iter().rev() {
                (cleanup.fun)();
            }
        }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(mut graph) = try_graph() {
            drop_scope(&mut graph, self.key);
        }
    }
}
