use unicode_general_category::{GeneralCategory, get_general_category};

pub(crate) struct LineBreak;

impl LineBreak {
    const PREFIX_CHARS: &'static str = "([\u{ff5b}\u{3014}\u{3008}\u{300a}\u{300c}\u{300e}\u{3010}\u{3018}\u{3016}\u{301d}\u{2018}\u{201c}\u{ff5f}\u{ab}$\u{2014}\u{2026}\u{2025}\u{3033}\u{3034}\u{3035}\\\u{ff3b}\u{ff08}{\u{a3}\u{a5}\"\u{3005}\u{3007}\u{ff04}\u{ffe5}\u{ffe6} #";
    const SUFFIX_CHARS: &'static str = ")]\u{ff5d}\u{3015}\u{3009}\u{300b}\u{300d}\u{300f}\u{3011}\u{3019}\u{3017}\u{301f}\u{2019}\u{201d}\u{ff60}\u{bb}\u{30fd}\u{30fe}\u{30fc}\u{30a1}\u{30a3}\u{30a5}\u{30a7}\u{30a9}\u{30c3}\u{30e3}\u{30e5}\u{30e7}\u{30ee}\u{30f5}\u{30f6}\u{3041}\u{3043}\u{3045}\u{3047}\u{3049}\u{3063}\u{3083}\u{3085}\u{3087}\u{308e}\u{3095}\u{3096}\u{31f0}\u{31f1}\u{31f2}\u{31f3}\u{31f4}\u{31f5}\u{31f6}\u{31f7}\u{31f8}\u{31f9}\u{31fa}\u{31fb}\u{31fc}\u{31fd}\u{31fe}\u{31ff}\u{3005}\u{303b}\u{2010}\u{30a0}\u{2013}\u{301c}?!\u{203c}\u{2047}\u{2048}\u{2049}\u{30fb}\u{3001}%,.:;\u{3002}\u{ff01}\u{ff1f}\u{ff3d}\u{ff09}\u{ff1a}\u{ff1b}\u{ff1d}}\u{a2}\u{b0}\"\u{2020}\u{2021}\u{2103}\u{3006}\u{ff05}\u{ff0c}\u{ff0e}";

    pub(crate) fn is_leading(code: u32) -> bool {
        Self::PREFIX_CHARS
            .chars()
            .any(|character| character as u32 == code)
    }

    pub(crate) fn is_following(code: u32) -> bool {
        Self::SUFFIX_CHARS
            .chars()
            .any(|character| character as u32 == code)
    }

    pub(crate) fn is_east_asian(code: u32) -> bool {
        let hangul = code > 0x1100 && code < 0x11ff
            || code > 0xA960 && code < 0xA97F
            || code > 0xAC00 && code < 0xD7FF;
        let cjk = code > 0x2E80 && code < 0x9FFF
            || code > 0xF900 && code < 0xFAFF
            || code > 0xFE30 && code < 0xFE4F
            || code > 0xFF00 && code < 0xFFEF;

        hangul || cjk
    }

    pub(crate) fn is_separator(code: u32) -> bool {
        Self::utf16_char(code).is_some_and(|character| {
            matches!(
                get_general_category(character),
                GeneralCategory::SpaceSeparator
                    | GeneralCategory::LineSeparator
                    | GeneralCategory::ParagraphSeparator
            )
        })
    }

    pub(crate) fn is_control(code: u32) -> bool {
        Self::utf16_char(code)
            .is_some_and(|character| get_general_category(character) == GeneralCategory::Control)
    }

    /// Character of the lower UTF-16 code unit.
    fn utf16_char(code: u32) -> Option<char> {
        char::from_u32(code & 0xFFFF)
    }
}

#[cfg(test)]
#[path = "tests/line_break.rs"]
mod tests;
