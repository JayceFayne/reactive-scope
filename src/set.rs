use smallvec::SmallVec;
use smallvec::{Drain, IntoIter};
use std::borrow::Borrow;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Set<T> {
    base: SmallVec<[T; 1]>,
}

impl<T> Default for Set<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for Set<T> {
    type Target = SmallVec<[T; 1]>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T> IntoIterator for Set<T> {
    type Item = T;

    type IntoIter = IntoIter<[T; 1]>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }
}

impl<T> Set<T> {
    pub const fn new() -> Self {
        Self {
            base: SmallVec::new_const(),
        }
    }

    pub fn drain(&mut self) -> Drain<'_, [T; 1]> {
        self.base.drain(..)
    }
}

impl<T> Set<T>
where
    T: Eq,
{
    pub fn insert(&mut self, k: T) {
        if self.contains(&k) {
            return;
        }
        self.base.push(k);
    }

    pub fn remove<Q>(&mut self, k: &Q)
    where
        Q: Eq + ?Sized,
        T: Borrow<Q>,
    {
        let pos = self
            .base
            .iter()
            .enumerate()
            .find(|&(_, e)| e.borrow().eq(k))
            .map(|(i, _)| i);
        self.base.remove(pos.unwrap());
    }
}

#[derive(Debug, Clone)]
pub struct DequeSet<T> {
    base: VecDeque<T>,
}

impl<T> Default for DequeSet<T> {
    fn default() -> Self {
        Self {
            base: VecDeque::new(),
        }
    }
}

impl<T> Deref for DequeSet<T> {
    type Target = VecDeque<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T> DequeSet<T>
where
    T: Eq,
{
    pub fn push(&mut self, k: T) {
        if self.contains(&k) {
            return;
        }
        self.base.push_back(k);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.base.pop_front()
    }

    pub fn clear(&mut self) {
        self.base.clear();
    }
}
