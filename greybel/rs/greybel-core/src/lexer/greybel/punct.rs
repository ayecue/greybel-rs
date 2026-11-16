use crate::Punct;

pub const PUNCTUATORS: &[Punct] = &[
    Punct::BitwiseAnd,
    Punct::BitwiseOr,
    Punct::ShiftLeft,
    Punct::ShiftRight,
    Punct::UnsignedShiftRight,
    Punct::Ellipsis,
    Punct::Arrow,
    Punct::Question,
    Punct::Coalesce,
    Punct::OptionalDot,
    Punct::Bang,
];
