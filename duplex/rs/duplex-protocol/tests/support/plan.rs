use duplex_protocol::{Bit, FieldsPlan, Plan, TypeContent};

pub const fn bit(byte: usize, mask: u8) -> Bit {
    Bit { byte, mask }
}

pub fn struct_fields_plan<'d>(plan: &'d Plan<'d>, index: usize) -> &'d FieldsPlan<'d> {
    match &plan.types[index].content {
        TypeContent::Struct(fields) => fields,
        content => panic!("a struct, not {content:?}"),
    }
}

pub const OLDER: &str = "use core 1.0; protocol P = \"p\" { open Login; client C {} provider D { message Sum { a: u8 } -> u8 } } struct Login { user: string }";
