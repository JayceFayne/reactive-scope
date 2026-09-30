use slotmap::SlotMap;
use smallbox::space::S4;
use smallbox::{SmallBox, smallbox};
use std::any::{Any, TypeId};
use std::mem;
use std::ops::{Index, IndexMut};
use std::task::Waker;

mod data;
mod debug;
mod keys;
mod scope;
mod signal;
mod state;
pub mod tls;

pub use debug::Snapshot;
pub use scope::Scope;
pub use signal::SignalGuard;

use data::{Batching, CleanUpData, ContextData, DebuggerCache, EffectData, ScopeData, SignalData};
use keys::{EffectKey, RawEffectKey, RawSignalKey, ScopeKey, SignalKey};
use signal::{RO, RW};
use state::{Effect as EffectState, EffectFlags, Graph as State, Scope as ScopeState, ScopeFlags};
use tls::graph;

pub type RawSignal<T, A> = signal::Signal<T, A>;
pub type Signal<T> = signal::Signal<T, signal::RW>;
pub type ReadSignal<T> = signal::Signal<T, signal::RO>;

#[derive(Debug)]
#[must_use = "have to use `run_in` in order to use"]
pub struct EffectGuard(EffectState);

impl EffectGuard {
    pub fn run_in<O, F: FnOnce() -> O>(self, fun: F) -> O {
        let ret = fun();
        graph().state.effect = self.0;
        ret
    }
}

pub struct ScopeGuard(ScopeState);

impl ScopeGuard {
    pub fn run_in<O, F: FnOnce() -> O>(self, fun: F) -> O {
        let ret = fun();
        graph().state.scope = self.0;
        ret
    }
}

#[must_use]
pub struct EffectRunner {
    cleanup: Option<Box<dyn FnOnce()>>,
    fun: SmallBox<dyn FnMut(), S4>,
    prev_effect: EffectState,
    prev_scope: Option<ScopeState>,
}

impl EffectRunner {
    pub fn run(mut self) {
        if let Some(cleanup) = self.cleanup {
            (cleanup)();
        }
        (self.fun)();
        let mut graph = graph();
        let effect = graph.state.effect.replace(self.prev_effect);
        if let Some(prev_scope) = self.prev_scope {
            graph.state.scope.replace(prev_scope);
        }
        graph[effect.key.unwrap()].fun = Some(self.fun);
    }
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn no_reactive_scope<T>() -> T {
    panic!("no reactive scope present");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn context_overwrite() -> ! {
    panic!("can't provide the same context twice");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn context_not_found<T>() -> T {
    panic!("can't find context");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn dropped_scope<T>() -> T {
    panic!("can't access dropped scope");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn attach_scope_to_itself() -> ! {
    panic!("can't attach scope to itself");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn no_tracking_effect() -> ! {
    panic!("tracking effect missing");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn notrack_violation() -> ! {
    panic!("reactive read in notrack scope");
}

#[cold]
#[cfg_attr(debug_assertions, track_caller)]
const fn nocreate_violation() -> ! {
    panic!("creation of reactive primitives is not allowed in nocreate scope");
}

pub struct Graph {
    scopes: SlotMap<ScopeKey, ScopeData>,
    state: State,
    debugger: DebuggerCache,
    batching: Batching,
}

impl Index<ScopeKey> for Graph {
    type Output = ScopeData;

    #[cfg_attr(debug_assertions, track_caller)]
    fn index(&self, scope_key: ScopeKey) -> &Self::Output {
        self.scopes.get(scope_key).unwrap_or_else(dropped_scope)
    }
}

impl IndexMut<ScopeKey> for Graph {
    #[cfg_attr(debug_assertions, track_caller)]
    fn index_mut(&mut self, scope_key: ScopeKey) -> &mut Self::Output {
        self.scopes.get_mut(scope_key).unwrap_or_else(dropped_scope)
    }
}

impl Index<SignalKey> for Graph {
    type Output = SignalData;

    #[cfg_attr(debug_assertions, track_caller)]
    fn index(&self, SignalKey(scope_key, signal_key): SignalKey) -> &Self::Output {
        &self[scope_key][signal_key]
    }
}

impl IndexMut<SignalKey> for Graph {
    #[cfg_attr(debug_assertions, track_caller)]
    fn index_mut(&mut self, SignalKey(scope_key, signal_key): SignalKey) -> &mut Self::Output {
        &mut self[scope_key][signal_key]
    }
}

impl Index<EffectKey> for Graph {
    type Output = EffectData;

    #[cfg_attr(debug_assertions, track_caller)]
    fn index(&self, EffectKey(scope_key, signal_key): EffectKey) -> &Self::Output {
        &self[scope_key][signal_key]
    }
}

impl IndexMut<EffectKey> for Graph {
    #[cfg_attr(debug_assertions, track_caller)]
    fn index_mut(&mut self, EffectKey(scope_key, signal_key): EffectKey) -> &mut Self::Output {
        &mut self[scope_key][signal_key]
    }
}

impl Graph {
    pub fn new() -> Self {
        Self {
            scopes: SlotMap::with_key(),
            state: State::new(),
            debugger: DebuggerCache::default(),
            batching: Batching::default(),
        }
    }

    pub fn effect_runner(&mut self, effect_key: EffectKey) -> EffectRunner {
        let (fun, mut reads, mut writes, cleanup, flags) = self[effect_key].take();
        let fun = fun.unwrap();
        for signal_key in reads.drain() {
            if let Some(scope) = self.scopes.get_mut(signal_key.0) {
                scope[signal_key.1].read_by.remove(&effect_key);
            }
        }
        for signal_key in writes.drain() {
            if let Some(scope) = self.scopes.get_mut(signal_key.0) {
                scope[signal_key.1].written_by.remove(&effect_key);
            }
        }
        let prev_effect = self.state.effect.replace(EffectState {
            key: Some(effect_key),
            flags: EffectFlags::empty(),
        });
        let prev_scope = Some(self.state.scope.replace(ScopeState {
            key: Some(effect_key.0),
            flags,
        }));
        let effect = &mut self[effect_key];
        effect.reads = reads;
        effect.writes = writes;
        EffectRunner {
            cleanup,
            fun,
            prev_effect,
            prev_scope,
        }
    }

    pub fn should_run(&mut self, signal_key: SignalKey, o_effect_key: EffectKey) -> bool {
        if self.batching.effect_cache.is_empty() {
            return true;
        }
        self.batching.signal_cache.clear();
        self.batching.signal_cache.push(signal_key);
        while let Some(signal_key) = self.batching.signal_cache.pop() {
            let Some(scope) = self.scopes.get(signal_key.0) else {
                continue;
            };
            let signal = &scope[signal_key.1];
            for &effect_key in signal.written_by.iter() {
                if o_effect_key == effect_key {
                    continue;
                }
                if self.batching.effect_cache.contains(&effect_key) {
                    return false;
                }
                let Some(scope) = self.scopes.get(effect_key.0) else {
                    continue;
                };
                let effect = &scope[effect_key.1];
                for &signal_key in effect.reads.iter() {
                    self.batching.signal_cache.push(signal_key);
                }
            }
        }
        true
    }

    pub fn pop_pending_effect(&mut self) -> Option<EffectKey> {
        while let Some(signal_key) = self.batching.signal_cache.pop() {
            let Some(scope) = self.scopes.get(signal_key.0) else {
                continue;
            };
            for &effect_key in scope[signal_key.1].read_by.iter() {
                self.batching.effect_cache.push(effect_key);
            }
        }
        while let Some(effect_key) = self.batching.effect_cache.pop() {
            if self.scopes.get(effect_key.0).is_none() {
                continue;
            }
            let mut should_run = true;
            let mut i = 0;
            while let Some(&signal_key) = self[effect_key].reads.get(i) {
                if !self.should_run(signal_key, effect_key) {
                    should_run = false;
                    break;
                }
                i += 1;
            }
            if should_run {
                return Some(effect_key);
            }
            self.batching.effect_cache.push(effect_key);
        }
        None
    }

    #[inline]
    pub fn waker(&mut self, waker: Waker) {
        self.batching.waker = Some(waker);
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_scope(&mut self) -> Scope {
        let current_scope = self.state.scope.key;
        let key = self.scopes.insert(ScopeData::new(current_scope));
        if let Some(parent) = current_scope {
            self[parent].children.insert(key);
        }
        Scope::new(key)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn track<T, A>(&mut self, signal: RawSignal<T, A>) {
        let signal_key = signal.key;
        let effect_state = self.state.effect;
        if let Some(effect_key) = effect_state.key {
            if effect_state.flags.contains(EffectFlags::Notrack) {
                notrack_violation();
            }
            if !effect_state.flags.contains(EffectFlags::Untrack) {
                self[signal_key].read_by.insert(effect_key);
                self[effect_key].reads.insert(signal_key);
            }
        } else {
            no_tracking_effect();
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn scope(&mut self, scope_key: ScopeKey) -> ScopeGuard {
        let prev = self
            .state
            .scope
            .take_mut(move |scope| scope.key = Some(scope_key));
        ScopeGuard(prev)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn nocreate(&mut self) -> ScopeGuard {
        let prev = self
            .state
            .scope
            .take_mut(move |scope| scope.flags.insert(ScopeFlags::Nocreate));
        ScopeGuard(prev)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn silent(&mut self) -> ScopeGuard {
        let prev = self
            .state
            .scope
            .take_mut(move |scope| scope.flags.insert(ScopeFlags::Silent));
        ScopeGuard(prev)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn untrack(&mut self) -> EffectGuard {
        debug_assert!(
            self.state.effect.key.is_some(),
            "untrack without tracking effect"
        );
        let prev = self
            .state
            .effect
            .take_mut(move |effect| effect.flags.insert(EffectFlags::Untrack));
        EffectGuard(prev)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn notrack(&mut self) -> EffectGuard {
        debug_assert!(
            self.state.effect.key.is_some(),
            "notrack without tracking effect"
        );
        let prev = self
            .state
            .effect
            .take_mut(move |effect| effect.flags.insert(EffectFlags::Notrack));
        EffectGuard(prev)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn notify<T, A>(&mut self, signal: RawSignal<T, A>) {
        let signal_key = signal.key;
        let effect_state = self.state.effect;
        if let Some(effect_key) = effect_state.key {
            self[signal_key].written_by.insert(effect_key);
            self[effect_key].writes.insert(signal_key);
            if effect_state.flags.contains(EffectFlags::Initial) {
                return;
            }
        }
        let scope_state = self.state.scope;
        if scope_state.flags.contains(ScopeFlags::Silent) {
            return;
        }
        let batching = &mut self.batching;
        if let Some(waker) = batching.waker.take() {
            waker.wake();
        }
        batching.signal_cache.push(signal_key);
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn detach(&mut self, scope_key: ScopeKey) {
        if let Some(parent_scope) = self[scope_key].parent.take() {
            self[parent_scope].children.remove(&scope_key);
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn attach(&mut self, scope_key: ScopeKey) {
        let current_scope = self.write_current_scope();
        if current_scope == scope_key {
            attach_scope_to_itself();
        }
        if let Some(parent_scope) = self[scope_key].parent.replace(current_scope) {
            self[parent_scope].children.remove(&scope_key);
        }
        self[current_scope].children.insert(scope_key);
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn read_current_scope(&self) -> ScopeKey {
        self.state.scope.key.unwrap_or_else(no_reactive_scope)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn write_current_scope(&self) -> ScopeKey {
        let scope = self.state.scope;
        if scope.flags.contains(ScopeFlags::Nocreate) {
            nocreate_violation();
        }
        scope.key.unwrap_or_else(no_reactive_scope)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_effect<O, F: FnMut() -> O + 'static>(&mut self, mut fun: F) -> EffectRunner {
        let current_scope = self.write_current_scope();
        let scope_flags = self.state.scope.flags;
        let effects = &mut self[current_scope].effects;
        let effect_key = EffectKey(current_scope, RawEffectKey(effects.len()));
        effects.push(EffectData::new(scope_flags));
        let prev_effect = self.state.effect.replace(EffectState {
            key: Some(effect_key),
            flags: EffectFlags::Initial,
        });
        EffectRunner {
            cleanup: None,
            fun: smallbox!(move || {
                fun();
            }),
            prev_effect,
            prev_scope: None,
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_memo<O: 'static, F: FnMut() -> O + 'static>(
        &mut self,
        mut fun: F,
    ) -> (EffectRunner, ReadSignal<O>) {
        let (rw, ro) = self.create_empty_signal::<O>().split();
        let effect_runner = self.create_effect(move || {
            rw.set_empty(fun());
        });
        (effect_runner, ro)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_selector<O: 'static + Eq, F: FnMut() -> O + 'static>(
        &mut self,
        mut fun: F,
    ) -> (EffectRunner, ReadSignal<O>) {
        let (rw, ro) = self.create_empty_signal::<O>().split();
        let effect_runner = self.create_effect(move || {
            let new = fun();
            if rw.with_empty_untracked(|v| v != Some(&new)) {
                rw.set_empty(new);
            }
        });
        (effect_runner, ro)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get<T: 'static + Copy, A>(&mut self, signal: RawSignal<T, A>) -> T {
        self.track(signal);
        self.get_untracked(signal)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_clone<T: 'static + Clone, A>(&mut self, signal: RawSignal<T, A>) -> T {
        self.track(signal);
        self.get_clone_untracked(signal)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with<T: 'static, U, A>(
        &mut self,
        signal: RawSignal<T, A>,
        fun: impl FnOnce(&T) -> U,
    ) -> U {
        self.track(signal);
        self.with_untracked(signal, fun)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with_empty_untracked<T: 'static, U, A>(
        &self,
        signal: RawSignal<T, A>,
        fun: impl FnOnce(Option<&T>) -> U,
    ) -> U {
        let signal_key = signal.key;
        let value = self[signal_key]
            .value
            .as_ref()
            .and_then(|v| v.downcast_ref());
        fun(value)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with_untracked<T: 'static, U, A>(
        &self,
        signal: RawSignal<T, A>,
        fun: impl FnOnce(&T) -> U,
    ) -> U {
        self.with_empty_untracked(signal, |val| fun(val.unwrap()))
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_clone_untracked<T: 'static + Clone, A>(&mut self, signal: RawSignal<T, A>) -> T {
        self.with_untracked(signal, Clone::clone)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_untracked<T: 'static + Copy, A>(&self, signal: RawSignal<T, A>) -> T {
        self.with_untracked(signal, |val| *val)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_ref_untracked<T, A, N>(&mut self, signal: RawSignal<T, A>) -> SignalGuard<T, N> {
        let signal_key = signal.key;
        let data = self[signal_key].value.take();
        SignalGuard::new(RawSignal::new(signal_key), data)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_ref<T, A>(&mut self, signal: RawSignal<T, A>) -> SignalGuard<T, RO> {
        self.track(signal);
        self.get_ref_untracked(signal)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_signal_with<T: Any>(&mut self, data: SignalData) -> Signal<T> {
        let current_scope = self.write_current_scope();
        let signals = &mut self[current_scope].signals;
        let signal_key = SignalKey(current_scope, RawSignalKey(signals.len()));
        signals.push(data);
        Signal::new(signal_key)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_signal<T: Any>(&mut self, val: T) -> Signal<T> {
        self.create_signal_with(SignalData::with(val))
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn create_empty_signal<T: Any>(&mut self) -> Signal<T> {
        self.create_signal_with(SignalData::empty())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn on_cleanup<O, F: FnOnce() -> O + 'static>(&mut self, fun: F) {
        let fun = Box::new(|| {
            fun();
        });
        if let Some(effect_key) = self.state.effect.key {
            self[effect_key].cleanup = Some(fun);
        } else {
            let current_scope = self.write_current_scope();
            self[current_scope].cleanup.push(CleanUpData::new(fun));
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn provide_context<T: Any>(&mut self, val: T) {
        let current_scope = self.write_current_scope();
        if self[current_scope]
            .context
            .insert(val.type_id(), ContextData::new(val))
            .is_some()
        {
            context_overwrite()
        }
    }

    fn search_context(
        &self,
        mut scope_key: ScopeKey,
        type_id: TypeId,
    ) -> Option<(ScopeKey, usize)> {
        loop {
            let scope = &self[scope_key];
            match scope.context.binary_search(&type_id).ok() {
                Some(index) => return Some((scope_key, index)),
                None if let Some(parent) = scope.parent => scope_key = parent,
                None => return None,
            }
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn try_use_context<T: Any + Clone>(&self) -> Option<T> {
        let (scope_key, index) =
            self.search_context(self.read_current_scope(), TypeId::of::<T>())?;
        Some(
            self[scope_key].context[index]
                .value
                .downcast_ref()
                .cloned()
                .unwrap(),
        )
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn use_context<T: Any + Clone>(&self) -> T {
        self.try_use_context().unwrap_or_else(context_not_found)
    }

    pub fn replace_empty<T: Any>(&mut self, signal: Signal<T>, val: T) -> Option<T> {
        self.notify(signal);
        let signal_key = signal.key;
        let signal = &mut self[signal_key];
        if let Some(data) = signal.value.as_mut().and_then(|v| v.downcast_mut()) {
            Some(mem::replace(data, val))
        } else {
            signal.value = Some(smallbox!(val));
            None
        }
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn update_silent<T: 'static, U>(
        &mut self,
        signal: Signal<T>,
        fun: impl FnOnce(&mut T) -> U,
    ) -> U {
        let signal_key = signal.key;
        let signal = &mut self[signal_key];
        fun(signal.value.as_mut().unwrap().downcast_mut().unwrap())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn replace_silent<T: Any>(&mut self, signal: Signal<T>, new: T) -> T {
        self.update_silent(signal, |val| mem::replace(val, new))
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn take_silent<T: Any + Default>(&mut self, signal: Signal<T>) -> T {
        self.replace_silent(signal, Default::default())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn update<T: 'static, U>(&mut self, signal: Signal<T>, fun: impl FnOnce(&mut T) -> U) -> U {
        self.notify(signal);
        self.update_silent(signal, fun)
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn replace<T: Any>(&mut self, signal: Signal<T>, new: T) -> T {
        self.update(signal, |val| mem::replace(val, new))
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn take<T: Any + Default>(&mut self, signal: Signal<T>) -> T {
        self.replace(signal, Default::default())
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut<T, A>(&mut self, signal: RawSignal<T, A>) -> SignalGuard<T, RW> {
        self.notify(signal);
        self.get_ref_untracked(signal)
    }
}
