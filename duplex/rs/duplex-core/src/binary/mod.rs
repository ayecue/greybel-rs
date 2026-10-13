mod borrow_decode;
mod codec;
mod impls;
mod reader;
mod validations;
mod values;
mod varint;
mod writer;

pub use borrow_decode::BorrowDecode;
pub use codec::{BinaryDecode, BinaryEncode};
pub use reader::BinaryReader;
pub use validations::{Pattern, Validatable, ValidatedDecode, ValidatedEncode, Validation};
pub use values::{Bytes, Fixed32, Fixed64};
pub use varint::Varuint;
pub(crate) use varint::{CONTINUATION, PAYLOAD};
pub use writer::BinaryWriter;
