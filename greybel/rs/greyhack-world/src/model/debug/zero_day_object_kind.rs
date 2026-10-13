/// Access granted by a zero-day partial object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroDayObjectKind {
    Computer,
    File,
}
