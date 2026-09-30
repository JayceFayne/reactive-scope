use super::{EffectKey, ScopeKey};
use bitflags::bitflags;
use std::mem;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct EffectFlags: u8 {
        const Initial = 1 << 0;
        const Untrack = 1 << 1;
        const Notrack = 1 << 2;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct ScopeFlags: u8 {
        const Nocreate = 1 << 0;
        const Silent = 1 << 1;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scope {
    pub key: Option<ScopeKey>,
    pub flags: ScopeFlags,
}

impl Scope {
    pub fn replace(&mut self, state: Self) -> Self {
        mem::replace(self, state)
    }

    pub fn take_mut<F: FnOnce(&mut Self) + 'static>(&mut self, fun: F) -> Self {
        let prev = *self;
        fun(self);
        prev
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Effect {
    pub key: Option<EffectKey>,
    pub flags: EffectFlags,
}

impl Effect {
    pub fn replace(&mut self, state: Self) -> Self {
        mem::replace(self, state)
    }

    pub fn take_mut<F: FnOnce(&mut Self) + 'static>(&mut self, fun: F) -> Self {
        let prev = *self;
        fun(self);
        prev
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Graph {
    pub scope: Scope,
    pub effect: Effect,
}

impl Graph {
    pub const fn new() -> Self {
        Self {
            scope: Scope {
                key: None,
                flags: ScopeFlags::empty(),
            },
            effect: Effect {
                key: None,
                flags: EffectFlags::empty(),
            },
        }
    }
}
