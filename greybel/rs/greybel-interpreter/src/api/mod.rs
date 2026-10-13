mod convert;
mod native;
pub(crate) mod sequence;

pub use self::convert::{FromArgument, FromArgumentMut, FromVm, IntoNativeResult};
pub use self::native::{AsyncNativeCall, Native, NativeCall, NativeDefault, NativeParam};
pub use self::sequence::{Sequence, SequenceItem};
use crate::vm::{ValueType, Vm};

const TYPE_COUNT: usize = 5;

pub type NativeWithName = (&'static str, &'static Native);

#[derive(Debug, Clone, Copy)]
pub struct Api {
    includes: &'static [Api],
    globals: &'static [NativeWithName],
    intrinsics: [&'static [NativeWithName]; TYPE_COUNT],
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

impl Api {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            includes: &[],
            globals: &[],
            intrinsics: [&[]; TYPE_COUNT],
        }
    }

    #[must_use]
    pub const fn set_includes(mut self, apis: &'static [Api]) -> Self {
        self.includes = apis;

        self
    }

    #[must_use]
    pub const fn set_globals(mut self, natives: &'static [NativeWithName]) -> Self {
        self.globals = natives;

        self
    }

    #[must_use]
    pub const fn set_intrinsics(
        mut self,
        kind: ValueType,
        natives: &'static [NativeWithName],
    ) -> Self {
        self.intrinsics[kind as usize] = natives;

        self
    }

    #[must_use]
    pub const fn globals(&self) -> &'static [NativeWithName] {
        self.globals
    }

    #[must_use]
    pub const fn intrinsics(&self, kind: ValueType) -> &'static [NativeWithName] {
        self.intrinsics[kind as usize]
    }

    #[must_use]
    pub const fn includes(&self) -> &'static [Api] {
        self.includes
    }
}

#[macro_export]
macro_rules! natives {
    ($($($name:literal =>)? $($constant:block =>)? $($segment:ident)::+),* $(,)?) => {
        {
            const NATIVES: &[$crate::NativeWithName] =
                &[$($crate::__native_with_name!($($name)? $($constant)? ; $($segment)::+)),*];
            NATIVES
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __native_with_name {
    (; $($segment:ident)::+) => {
        ($crate::__last_segment!($($segment)::+), &$($segment)::+::NATIVE)
    };
    ($name:literal ; $($segment:ident)::+) => {
        ($name, &$($segment)::+::NATIVE)
    };
    ($constant:block ; $($segment:ident)::+) => {
        ($constant, &$($segment)::+::NATIVE)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __last_segment {
    ($last:ident) => {
        stringify!($last)
    };
    ($first:ident :: $($rest:ident)::+) => {
        $crate::__last_segment!($($rest)::+)
    };
}

pub trait Register {
    fn register(&self, vm: &mut Vm);
}

impl Register for Api {
    fn register(&self, vm: &mut Vm) {
        for api in self.includes {
            api.register(vm);
        }

        for &(name, native) in self.globals {
            let value = vm.native_function(native);

            vm.define(name, value);
        }

        for kind in ValueType::ALL {
            for &(name, native) in self.intrinsics[kind as usize] {
                let value = vm.native_function(native);

                vm.add_intrinsic(kind, name, value);
            }
        }
    }
}

impl<F: Fn(&mut Vm)> Register for F {
    fn register(&self, vm: &mut Vm) {
        self(vm);
    }
}
