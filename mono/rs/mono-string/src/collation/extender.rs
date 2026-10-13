const HORIZONTAL_BAR: i32 = 0x2015;
const IDEOGRAPHIC_ITERATION_MARK: i32 = 0x3005;
const VERTICAL_KANA_REPEAT_MARK: i32 = 0x3031;
const VERTICAL_KANA_REPEAT_WITH_VOICED_SOUND_MARK: i32 = 0x3032;
const HIRAGANA_ITERATION_MARK: i32 = 0x309D;
const HIRAGANA_VOICED_ITERATION_MARK: i32 = 0x309E;
const PROLONGED_SOUND_MARK: i32 = 0x30FC;
const KATAKANA_ITERATION_MARK: i32 = 0x30FD;
const KATAKANA_VOICED_ITERATION_MARK: i32 = 0x30FE;
const ARABIC_SHADDA_ISOLATED_FORM: i32 = 0xFE7C;
const ARABIC_SHADDA_MEDIAL_FORM: i32 = 0xFE7D;
const HALFWIDTH_PROLONGED_SOUND_MARK: i32 = 0xFF70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Extender {
    None,
    Simple,
    Dakuten,
    Conditional,
    Buggy,
}

pub(super) fn dash_type(extender: Extender) -> i32 {
    match extender {
        Extender::None => 3,
        Extender::Conditional => 5,
        _ => 4,
    }
}

pub(super) fn extender_type(unit: i32) -> Extender {
    if unit == HORIZONTAL_BAR
        || !(IDEOGRAPHIC_ITERATION_MARK..=HALFWIDTH_PROLONGED_SOUND_MARK).contains(&unit)
    {
        return Extender::None;
    }

    match unit {
        ARABIC_SHADDA_ISOLATED_FORM | ARABIC_SHADDA_MEDIAL_FORM => Extender::Simple,
        HALFWIDTH_PROLONGED_SOUND_MARK => Extender::Conditional,
        _ if unit > KATAKANA_VOICED_ITERATION_MARK => Extender::None,
        IDEOGRAPHIC_ITERATION_MARK => Extender::Buggy,
        VERTICAL_KANA_REPEAT_MARK
        | VERTICAL_KANA_REPEAT_WITH_VOICED_SOUND_MARK
        | HIRAGANA_ITERATION_MARK
        | KATAKANA_ITERATION_MARK => Extender::Simple,
        HIRAGANA_VOICED_ITERATION_MARK | KATAKANA_VOICED_ITERATION_MARK => Extender::Dakuten,
        PROLONGED_SOUND_MARK => Extender::Conditional,
        _ => Extender::None,
    }
}
