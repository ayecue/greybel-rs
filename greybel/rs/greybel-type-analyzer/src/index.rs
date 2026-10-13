macro_rules! index_type {
    ($($(#[$meta:meta])* $name:ident: $label:literal;)*) => {$(
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(::std::num::NonZeroU32);

        impl $name {
            pub(crate) const fn from_u32(index: u32) -> Self {
                match ::std::num::NonZeroU32::new(index.wrapping_add(1)) {
                    Some(raw) => Self(raw),
                    None => panic!(concat!("too many ", $label)),
                }
            }

            pub(crate) fn new(index: usize) -> Self {
                Self::from_u32(u32::try_from(index).expect(concat!("too many ", $label)))
            }

            pub(crate) fn index(self) -> usize {
                (self.0.get() - 1) as usize
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.index()).finish()
            }
        }
    )*};
}

pub(crate) use index_type;
