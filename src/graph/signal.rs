use super::keys::SignalKey;
use super::tls::{graph, try_graph};
use core::fmt;
use smallbox::SmallBox;
use smallbox::space::S4;
use std::any::Any;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref,
    DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr,
    ShrAssign, Sub, SubAssign,
};

#[derive(Debug)]
pub struct RO;
#[derive(Debug)]
pub struct RW;

#[must_use = "signals do nothing unless read or written to"]
pub struct Signal<T, A> {
    pub(crate) key: SignalKey,
    t_marker: PhantomData<T>,
    a_marker: PhantomData<A>,
    s_maker: PhantomData<*mut ()>,
}

impl<T, A> Clone for Signal<T, A> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, A> Copy for Signal<T, A> {}

impl<T, A> Signal<T, A> {
    pub(crate) const fn new(signal_key: SignalKey) -> Self {
        Self {
            key: signal_key,
            t_marker: PhantomData,
            a_marker: PhantomData,
            s_maker: PhantomData,
        }
    }
}

impl<T: 'static + Copy, A> Signal<T, A> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get(self) -> T {
        graph().get(self)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_untracked(self) -> T {
        graph().get_untracked(self)
    }
}

#[cfg(nightly)]
impl<T: Copy + 'static, A> FnOnce<()> for Signal<T, A> {
    type Output = T;

    #[inline]
    extern "rust-call" fn call_once(self, _args: ()) -> Self::Output {
        self.get()
    }
}

impl<T: 'static, A> Signal<T, A> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn track(self) {
        graph().track(self);
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with<U>(self, fun: impl FnOnce(&T) -> U) -> U {
        graph().with(self, fun)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn with_untracked<U>(self, fun: impl FnOnce(&T) -> U) -> U {
        graph().with_untracked(self, fun)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_ref(self) -> SignalGuard<T, RO> {
        graph().get_ref(self)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_ref_untracked(self) -> SignalGuard<T, RO> {
        graph().get_ref_untracked(self)
    }

    #[inline]
    pub const fn readonly(self) -> Signal<T, RO> {
        Signal::new(self.key)
    }
}

impl<T: 'static> Signal<T, RW> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn notify(self) {
        graph().notify(self);
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut(self) -> SignalGuard<T, RW> {
        graph().get_mut(self)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_mut_silent(self) -> SignalGuard<T, RW> {
        graph().get_ref_untracked(self)
    }

    pub(crate) fn with_empty_untracked<U>(self, fun: impl FnOnce(Option<&T>) -> U) -> U {
        graph().with_empty_untracked(self, fun)
    }

    pub(crate) fn set_empty(self, val: T) {
        let val = graph().replace_empty(self, val);
        drop(val);
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn update_silent<U>(self, fun: impl FnOnce(&mut T) -> U) -> U {
        graph().update_silent(self, fun)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn update<U>(self, fun: impl FnOnce(&mut T) -> U) -> U {
        graph().update(self, fun)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set(self, val: T) {
        let val = graph().replace(self, val);
        drop(val);
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set_silent(self, val: T) {
        let val = graph().replace_silent(self, val);
        drop(val);
    }
}

impl<T> Signal<T, RW> {
    #[inline]
    pub const fn split(self) -> (Signal<T, RW>, Signal<T, RO>) {
        (Signal::new(self.key), Signal::new(self.key))
    }
}

impl<T: 'static + Clone, A> Signal<T, A> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn get_clone(self) -> T {
        graph().get_clone(self)
    }
}

impl<T: 'static + Default> Signal<T, RW> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn take_silent(self) -> T {
        graph().take_silent(self)
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn take(self) -> T {
        graph().take(self)
    }
}

#[cfg(nightly)]
impl<T: 'static> FnOnce<(T,)> for Signal<T, RW> {
    type Output = ();

    extern "rust-call" fn call_once(self, (val,): (T,)) -> Self::Output {
        self.set(val);
    }
}

pub struct SignalGuard<T, A> {
    data: Option<SmallBox<dyn Any, S4>>,
    inner: Signal<T, A>,
}

impl<T: 'static + Debug, A> fmt::Debug for SignalGuard<T, A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T, A> SignalGuard<T, A> {
    pub(crate) const fn new(inner: Signal<T, A>, data: Option<SmallBox<dyn Any, S4>>) -> Self {
        Self { data, inner }
    }
}

impl<T: 'static, A> Deref for SignalGuard<T, A> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data.as_ref().unwrap().downcast_ref().unwrap()
    }
}

impl<T: 'static> DerefMut for SignalGuard<T, RW> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().unwrap().downcast_mut().unwrap()
    }
}

impl<T, A> Drop for SignalGuard<T, A> {
    fn drop(&mut self) {
        if let Some(mut graph) = try_graph() {
            graph[self.inner.key].value = self.data.take();
        }
    }
}

macro_rules! impl_fmt {
    ($trait:ident) => {
        impl<T, A> fmt::$trait for Signal<T, A>
        where
            T: fmt::$trait + 'static,
        {
            #[inline]
            #[cfg_attr(debug_assertions, track_caller)]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.with(|value| value.fmt(f))
            }
        }
    };
}

impl_fmt!(Debug);
impl_fmt!(Display);
impl_fmt!(Binary);
impl_fmt!(Octal);
impl_fmt!(LowerHex);
impl_fmt!(UpperHex);
impl_fmt!(LowerExp);
impl_fmt!(UpperExp);

macro_rules! impl_op {
    ($trait:ident, $method:ident) => {
        impl<T, A> $trait<T> for Signal<T, A>
        where
            T: $trait<Output = T> + 'static + Copy,
        {
            type Output = T;

            #[inline]
            #[cfg_attr(debug_assertions, track_caller)]
            fn $method(self, other: T) -> Self::Output {
                self.with(|val| val.$method(other))
            }
        }
    };
}

macro_rules! impl_unary_op {
    ($trait:ident, $method:ident) => {
        impl<T, A> $trait for Signal<T, A>
        where
            T: $trait<Output = T> + 'static + Copy,
        {
            type Output = T;

            #[inline]
            #[cfg_attr(debug_assertions, track_caller)]
            fn $method(self) -> Self::Output {
                self.with(|val| val.$method())
            }
        }
    };
}

macro_rules! impl_assign_op {
    ($trait:ident, $method:ident) => {
        impl<T, Rhs> $trait<Rhs> for Signal<T, RW>
        where
            T: $trait<Rhs> + 'static,
        {
            #[inline]
            #[cfg_attr(debug_assertions, track_caller)]
            fn $method(&mut self, rhs: Rhs) {
                self.update(|val| val.$method(rhs));
            }
        }
    };
}

impl_op!(Add, add);
impl_assign_op!(AddAssign, add_assign);
impl_op!(Sub, sub);
impl_assign_op!(SubAssign, sub_assign);
impl_op!(Mul, mul);
impl_assign_op!(MulAssign, mul_assign);
impl_unary_op!(Neg, neg);
impl_unary_op!(Not, not);
impl_op!(Div, div);
impl_assign_op!(DivAssign, div_assign);
impl_op!(Rem, rem);
impl_assign_op!(RemAssign, rem_assign);
impl_op!(BitAnd, bitand);
impl_assign_op!(BitAndAssign, bitand_assign);
impl_op!(BitOr, bitor);
impl_assign_op!(BitOrAssign, bitor_assign);
impl_op!(BitXor, bitxor);
impl_assign_op!(BitXorAssign, bitxor_assign);
impl_op!(Shl, shl);
impl_assign_op!(ShlAssign, shl_assign);
impl_op!(Shr, shr);
impl_assign_op!(ShrAssign, shr_assign);

macro_rules! impl_partial_cmp {
    ($trait:ident, $method:ident, $return:ty) => {
        impl<T, A, Rhs> $trait<Rhs> for Signal<T, A>
        where
            T: $trait<Rhs> + 'static,
        {
            #[inline]
            #[cfg_attr(debug_assertions, track_caller)]
            fn $method(&self, other: &Rhs) -> $return {
                self.with(|value| value.$method(other))
            }
        }
    };
}

impl_partial_cmp!(PartialEq, eq, bool);
impl_partial_cmp!(PartialOrd, partial_cmp, Option<Ordering>);
