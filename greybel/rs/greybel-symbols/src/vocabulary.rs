macro_rules! vocabulary {
    ($name:ident { $($variant:ident => $text:literal,)* }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum $name {
            $($variant,)*
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];
            pub const COUNT: usize = Self::ALL.len();

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }

            #[must_use]
            pub const fn index(self) -> usize {
                self as usize
            }

            #[must_use]
            pub fn parse(text: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|symbol| symbol.as_str() == text)
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

pub(crate) use vocabulary;
