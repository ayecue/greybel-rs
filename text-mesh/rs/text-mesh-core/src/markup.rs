use crate::hash::NameHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tag {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Mark,
    Subscript,
    Superscript,
    FontWeight,
    Position,
    VerticalOffset,
    Page,
    NoBreak,
    Size,
    Font,
    Material,
    Space,
    Alpha,
    Link,
    Align,
    Width,
    Color,
    Gradient,
    CharacterSpace,
    Monospace,
    Indent,
    LineIndent,
    Sprite,
    Lowercase,
    Uppercase,
    SmallCaps,
    Margin,
    MarginLeft,
    MarginRight,
    LineHeight,
    NoParse,
    Action,
    Scale,
    Rotate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Markup {
    Open(Tag),
    Close(Tag),
    NoEffect,
}

macro_rules! markups {
    ($($name:literal => $markup:expr),* $(,)?) => {
        &[$((NameHash::of($name), $markup), (NameHash::of_upper($name), $markup)),*]
    };
}

impl Markup {
    const BOTH_CASES: &[(i32, Self)] = markups! {
        "b" => Self::Open(Tag::Bold),
        "/b" => Self::Close(Tag::Bold),
        "i" => Self::Open(Tag::Italic),
        "/i" => Self::Close(Tag::Italic),
        "s" => Self::Open(Tag::Strikethrough),
        "/s" => Self::Close(Tag::Strikethrough),
        "u" => Self::Open(Tag::Underline),
        "/u" => Self::Close(Tag::Underline),
        "mark" => Self::Open(Tag::Mark),
        "/mark" => Self::Close(Tag::Mark),
        "sub" => Self::Open(Tag::Subscript),
        "/sub" => Self::Close(Tag::Subscript),
        "sup" => Self::Open(Tag::Superscript),
        "/sup" => Self::Close(Tag::Superscript),
        "font-weight" => Self::Open(Tag::FontWeight),
        "/font-weight" => Self::Close(Tag::FontWeight),
        "pos" => Self::Open(Tag::Position),
        "/pos" => Self::Close(Tag::Position),
        "voffset" => Self::Open(Tag::VerticalOffset),
        "/voffset" => Self::Close(Tag::VerticalOffset),
        "page" => Self::Open(Tag::Page),
        "nobr" => Self::Open(Tag::NoBreak),
        "/nobr" => Self::Close(Tag::NoBreak),
        "size" => Self::Open(Tag::Size),
        "/size" => Self::Close(Tag::Size),
        "font" => Self::Open(Tag::Font),
        "/font" => Self::Close(Tag::Font),
        "material" => Self::Open(Tag::Material),
        "/material" => Self::Close(Tag::Material),
        "space" => Self::Open(Tag::Space),
        "alpha" => Self::Open(Tag::Alpha),
        "link" => Self::Open(Tag::Link),
        "/link" => Self::Close(Tag::Link),
        "align" => Self::Open(Tag::Align),
        "/align" => Self::Close(Tag::Align),
        "width" => Self::Open(Tag::Width),
        "/width" => Self::Close(Tag::Width),
        "color" => Self::Open(Tag::Color),
        "/color" => Self::Close(Tag::Color),
        "gradient" => Self::Open(Tag::Gradient),
        "/gradient" => Self::Close(Tag::Gradient),
        "cspace" => Self::Open(Tag::CharacterSpace),
        "/cspace" => Self::Close(Tag::CharacterSpace),
        "mspace" => Self::Open(Tag::Monospace),
        "/mspace" => Self::Close(Tag::Monospace),
        "indent" => Self::Open(Tag::Indent),
        "/indent" => Self::Close(Tag::Indent),
        "line-indent" => Self::Open(Tag::LineIndent),
        "/line-indent" => Self::Close(Tag::LineIndent),
        "sprite" => Self::Open(Tag::Sprite),
        "lowercase" => Self::Open(Tag::Lowercase),
        "/lowercase" => Self::Close(Tag::Lowercase),
        "uppercase" => Self::Open(Tag::Uppercase),
        "/uppercase" => Self::Close(Tag::Uppercase),
        "allcaps" => Self::Open(Tag::Uppercase),
        "/allcaps" => Self::Close(Tag::Uppercase),
        "smallcaps" => Self::Open(Tag::SmallCaps),
        "/smallcaps" => Self::Close(Tag::SmallCaps),
        "margin" => Self::Open(Tag::Margin),
        "/margin" => Self::Close(Tag::Margin),
        "margin-left" => Self::Open(Tag::MarginLeft),
        "margin-right" => Self::Open(Tag::MarginRight),
        "line-height" => Self::Open(Tag::LineHeight),
        "/line-height" => Self::Close(Tag::LineHeight),
        "noparse" => Self::Open(Tag::NoParse),
        "/noparse" => Self::Close(Tag::NoParse),
        "action" => Self::Open(Tag::Action),
        "/action" => Self::Close(Tag::Action),
        "scale" => Self::Open(Tag::Scale),
        "/scale" => Self::Close(Tag::Scale),
        "rotate" => Self::Open(Tag::Rotate),
        "/rotate" => Self::Close(Tag::Rotate),
        "/table" => Self::NoEffect,
        "tr" => Self::NoEffect,
        "/tr" => Self::NoEffect,
        "th" => Self::NoEffect,
        "/th" => Self::NoEffect,
    };
    const LOWERCASE_ONLY: &[(i32, Self)] = &[(NameHash::of("/a"), Self::NoEffect)];

    pub(crate) fn from_name(name: i32) -> Option<Self> {
        Self::BOTH_CASES
            .iter()
            .chain(Self::LOWERCASE_ONLY)
            .find(|(hash, _)| *hash == name)
            .map(|&(_, markup)| markup)
    }
}
