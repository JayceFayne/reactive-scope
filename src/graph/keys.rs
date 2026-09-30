use core::fmt::{Debug, Formatter, Result};
use slotmap::new_key_type;

new_key_type! { pub struct ScopeKey; }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawSignalKey(pub usize);

impl Debug for RawSignalKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignalKey(pub ScopeKey, pub RawSignalKey);

impl Debug for SignalKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "r{:?}s{:?}", self.0.0, self.1)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawEffectKey(pub usize);

impl Debug for RawEffectKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EffectKey(pub ScopeKey, pub RawEffectKey);

impl Debug for EffectKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "r{:?}e{:?}", self.0.0, self.1)
    }
}
