use super::Graph;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};

thread_local! {
    static GRAPH: Cell<Option<Graph>> = Cell::new(Some(Graph::new()));
}

#[must_use = "deref in order to access the graph"]
pub struct GraphGuard {
    graph: Option<Graph>,
}

impl Drop for GraphGuard {
    #[inline]
    fn drop(&mut self) {
        GRAPH.set(self.graph.take());
    }
}

impl Deref for GraphGuard {
    type Target = Graph;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.graph.as_ref().unwrap()
    }
}

impl DerefMut for GraphGuard {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.graph.as_mut().unwrap()
    }
}

#[inline]
pub fn try_graph() -> Option<GraphGuard> {
    Some(GraphGuard {
        graph: Some(GRAPH.try_with(Cell::take).ok()??),
    })
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn reentrant_acquisition<T>() -> T {
    panic!("reentrant reactive graph acquisition");
}

#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn graph() -> GraphGuard {
    try_graph().unwrap_or_else(reentrant_acquisition)
}
