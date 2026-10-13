use crate::vocabulary::vocabulary;

pub const TAG_PREFIX: char = '@';

vocabulary!(Tag {
    Param => "param",
    Params => "params",
    Return => "return",
    Returns => "returns",
    Mutates => "mutates",
    Define => "define",
    Type => "type",
    VirtualType => "vtype",
    Template => "template",
    Extends => "extends",
    Property => "property",
    Prop => "prop",
    Function => "function",
    Description => "description",
    Example => "example",
});

pub const ANY: &str = "any";
pub const UNKNOWN: &str = "unknown";
pub const BOOL: &str = "bool";
pub const BOOLEAN: &str = "boolean";
pub const NIL: &str = "nil";
