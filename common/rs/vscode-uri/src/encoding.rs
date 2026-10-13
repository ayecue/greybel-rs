use std::borrow::Cow;
use std::fmt::Write;

pub(crate) struct PercentEncoding;

impl PercentEncoding {
    fn escape_sequence(character: char) -> Option<&'static str> {
        Some(match character {
            ':' => "%3A",
            '/' => "%2F",
            '?' => "%3F",
            '#' => "%23",
            '[' => "%5B",
            ']' => "%5D",
            '@' => "%40",
            '!' => "%21",
            '$' => "%24",
            '&' => "%26",
            '\'' => "%27",
            '(' => "%28",
            ')' => "%29",
            '*' => "%2A",
            '+' => "%2B",
            ',' => "%2C",
            ';' => "%3B",
            '=' => "%3D",
            ' ' => "%20",
            _ => return None,
        })
    }

    fn is_verbatim(character: char, is_path: bool, is_authority: bool) -> bool {
        character.is_ascii_alphanumeric()
            || matches!(character, '-' | '.' | '_' | '~')
            || (is_path && character == '/')
            || (is_authority && matches!(character, '[' | ']' | ':'))
    }

    pub(crate) fn encode(component: &str, is_path: bool, is_authority: bool) -> Cow<'_, str> {
        let Some(first) = component
            .char_indices()
            .find(|&(_, character)| !Self::is_verbatim(character, is_path, is_authority))
            .map(|(index, _)| index)
        else {
            return Cow::Borrowed(component);
        };
        let mut escaped_text = String::with_capacity(component.len() + 8);

        escaped_text.push_str(&component[..first]);
        for character in component[first..].chars() {
            if Self::is_verbatim(character, is_path, is_authority) {
                escaped_text.push(character);
            } else if let Some(escape) = Self::escape_sequence(character) {
                escaped_text.push_str(escape);
            } else {
                let mut bytes = [0; 4];

                for byte in character.encode_utf8(&mut bytes).bytes() {
                    let _ = write!(escaped_text, "%{byte:02X}");
                }
            }
        }

        Cow::Owned(escaped_text)
    }

    pub(crate) fn encode_minimal(path: &str) -> Cow<'_, str> {
        if !path.contains(['#', '?']) {
            return Cow::Borrowed(path);
        }

        let mut escaped_text = String::with_capacity(path.len() + 4);

        for character in path.chars() {
            match character {
                '#' => escaped_text.push_str("%23"),
                '?' => escaped_text.push_str("%3F"),
                character => escaped_text.push(character),
            }
        }

        Cow::Owned(escaped_text)
    }

    pub(crate) fn decode(text: &str) -> Cow<'_, str> {
        let bytes = text.as_bytes();
        let is_escape = |index: usize| {
            bytes.get(index) == Some(&b'%')
                && bytes.get(index + 1).is_some_and(u8::is_ascii_alphanumeric)
                && bytes.get(index + 2).is_some_and(u8::is_ascii_alphanumeric)
        };

        if !(0..bytes.len()).any(is_escape) {
            return Cow::Borrowed(text);
        }

        let mut output = String::with_capacity(text.len());
        let mut copy_position = 0;
        let mut index = 0;

        while index < bytes.len() {
            if !is_escape(index) {
                index += 1;
                continue;
            }

            let start = index;

            while is_escape(index) {
                index += 3;
            }

            output.push_str(&text[copy_position..start]);
            Self::decode_run(&text[start..index], &mut output);
            copy_position = index;
        }

        output.push_str(&text[copy_position..]);

        Cow::Owned(output)
    }

    fn decode_run(mut run: &str, output: &mut String) {
        loop {
            if let Some(text) = Self::decode_component(run) {
                output.push_str(&text);

                return;
            }

            if run.len() <= 3 {
                output.push_str(run);

                return;
            }

            output.push_str(&run[..3]);
            run = &run[3..];
        }
    }

    fn decode_component(run: &str) -> Option<String> {
        let bytes = run
            .as_bytes()
            .chunks(3)
            .map(|escape| {
                let hex = std::str::from_utf8(escape.get(1..3)?).ok()?;

                u8::from_str_radix(hex, 16).ok()
            })
            .collect::<Option<Vec<u8>>>()?;

        String::from_utf8(bytes).ok()
    }
}
