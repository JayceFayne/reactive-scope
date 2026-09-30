use super::keys::{EffectKey, RawEffectKey, RawSignalKey, ScopeKey, SignalKey};
use super::state::ScopeFlags;
use crate::graph::debug::Indent;
use crate::set::{DequeSet, Set};
use smallbox::space::S4;
use smallbox::{SmallBox, smallbox};
use std::any::{Any, TypeId};
use std::ops::{Index, IndexMut};
#[cfg(debug_assertions)]
use std::panic::Location;
use std::task::Waker;
use vec_btree_map::VecBTreeMap;

pub struct SignalData {
    pub value: Option<SmallBox<dyn Any, S4>>,
    pub read_by: Set<EffectKey>,
    pub written_by: Set<EffectKey>,
    #[cfg(debug_assertions)]
    pub location: &'static Location<'static>,
}

impl SignalData {
    #[cfg_attr(debug_assertions, track_caller)]
    pub const fn new(val: Option<SmallBox<dyn Any, S4>>) -> Self {
        Self {
            value: val,
            read_by: Set::new(),
            written_by: Set::new(),
            #[cfg(debug_assertions)]
            location: Location::caller(),
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with<T: Any>(val: T) -> Self {
        Self::new(Some(smallbox!(val)))
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub const fn empty() -> Self {
        Self::new(None)
    }
}

pub struct EffectData {
    pub fun: Option<SmallBox<dyn FnMut(), S4>>,
    pub reads: Set<SignalKey>,
    pub writes: Set<SignalKey>,
    pub cleanup: Option<Box<dyn FnOnce()>>,
    pub scope_flags: ScopeFlags,
    #[cfg(debug_assertions)]
    pub location: &'static Location<'static>,
}

impl EffectData {
    #[cfg_attr(debug_assertions, track_caller)]
    pub const fn new(scope_flags: ScopeFlags) -> Self {
        Self {
            fun: None,
            reads: Set::new(),
            writes: Set::new(),
            cleanup: None,
            scope_flags,
            #[cfg(debug_assertions)]
            location: Location::caller(),
        }
    }

    #[allow(clippy::type_complexity)]
    pub fn take(
        &mut self,
    ) -> (
        Option<SmallBox<dyn FnMut(), S4>>,
        Set<SignalKey>,
        Set<SignalKey>,
        Option<Box<dyn FnOnce()>>,
        ScopeFlags,
    ) {
        (
            self.fun.take(),
            std::mem::take(&mut self.reads),
            std::mem::take(&mut self.writes),
            self.cleanup.take(),
            self.scope_flags,
        )
    }
}

pub struct ContextData {
    pub value: SmallBox<dyn Any, S4>,
}

impl ContextData {
    pub fn new<T: Any>(val: T) -> Self {
        Self {
            value: smallbox!(val),
        }
    }
}

pub struct CleanUpData {
    pub fun: Box<dyn FnOnce()>,
}

impl CleanUpData {
    pub const fn new(fun: Box<dyn FnOnce()>) -> Self {
        Self { fun }
    }
}

#[derive(Debug, Default)]
pub struct Batching {
    pub signal_cache: DequeSet<SignalKey>,
    pub effect_cache: DequeSet<EffectKey>,
    pub scope_stack: Vec<ScopeKey>,
    pub waker: Option<Waker>,
}

#[derive(Default)]
pub struct DebuggerCache {
    pub scope_stack: Vec<(ScopeKey, Indent, bool)>,
}

pub struct ScopeData {
    pub parent: Option<ScopeKey>,
    pub signals: Vec<SignalData>,
    pub effects: Vec<EffectData>,
    pub cleanup: Vec<CleanUpData>,
    pub context: VecBTreeMap<TypeId, ContextData>,
    pub children: Set<ScopeKey>,
}

impl ScopeData {
    pub const fn new(parent: Option<ScopeKey>) -> Self {
        Self {
            parent,
            signals: Vec::new(),
            effects: Vec::new(),
            cleanup: Vec::new(),
            context: VecBTreeMap::new(),
            children: Set::new(),
        }
    }
}

impl Index<RawEffectKey> for ScopeData {
    type Output = EffectData;

    #[cfg_attr(debug_assertions, track_caller)]
    fn index(&self, RawEffectKey(effect_key): RawEffectKey) -> &Self::Output {
        &self.effects[effect_key]
    }
}

impl IndexMut<RawEffectKey> for ScopeData {
    #[cfg_attr(debug_assertions, track_caller)]
    fn index_mut(&mut self, RawEffectKey(effect_key): RawEffectKey) -> &mut Self::Output {
        &mut self.effects[effect_key]
    }
}

impl Index<RawSignalKey> for ScopeData {
    type Output = SignalData;

    #[cfg_attr(debug_assertions, track_caller)]
    fn index(&self, RawSignalKey(signal_key): RawSignalKey) -> &Self::Output {
        &self.signals[signal_key]
    }
}

impl IndexMut<RawSignalKey> for ScopeData {
    #[cfg_attr(debug_assertions, track_caller)]
    fn index_mut(&mut self, RawSignalKey(signal_key): RawSignalKey) -> &mut Self::Output {
        &mut self.signals[signal_key]
    }
}
