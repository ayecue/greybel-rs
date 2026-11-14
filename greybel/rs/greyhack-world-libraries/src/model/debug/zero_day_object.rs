use super::ZeroDayObjectKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroDayObject {
    pub kind: ZeroDayObjectKind,
    pub methods: Vec<String>,
    pub access: String,
}
