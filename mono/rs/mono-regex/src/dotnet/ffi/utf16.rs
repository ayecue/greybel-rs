/// Library string input containing UTF-16 data and an `int` length.
pub struct Utf16 {
    pub(super) units: Vec<u16>,
    pub(super) length: i32,
}

impl Utf16 {
    /// Returns `None` if the text exceeds the .NET string length limit.
    pub fn new(text: &str) -> Option<Self> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let length = i32::try_from(units.len()).ok()?;

        Some(Self { units, length })
    }
}
