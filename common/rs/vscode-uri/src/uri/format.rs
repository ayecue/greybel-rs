use std::borrow::Cow;

use super::Uri;
use crate::encoding::PercentEncoding;

impl Uri {
    pub(super) fn format(&self, skip_encoding: bool) -> String {
        let encode = |text: &'_ str, is_path: bool, is_authority: bool| -> String {
            let escaped_piece: Cow<'_, str> = if skip_encoding {
                PercentEncoding::encode_minimal(text)
            } else {
                PercentEncoding::encode(text, is_path, is_authority)
            };

            escaped_piece.into_owned()
        };
        let mut output = String::new();

        if !self.components.scheme.is_empty() {
            output.push_str(&self.components.scheme);
            output.push(':');
        }

        if !self.components.authority.is_empty()
            || self.components.scheme == Self::FILE
            || self.components.scheme == Self::MEMORY
        {
            output.push_str("//");
        }

        if !self.components.authority.is_empty() {
            let mut authority = self.components.authority.as_str();

            if let Some(at) = authority.find('@') {
                let userinfo = &authority[..at];

                authority = &authority[at + 1..];
                match userinfo.rfind(':') {
                    None => output.push_str(&encode(userinfo, false, false)),
                    Some(colon) => {
                        output.push_str(&encode(&userinfo[..colon], false, false));
                        output.push(':');
                        output.push_str(&encode(&userinfo[colon + 1..], false, true));
                    }
                }

                output.push('@');
            }

            let authority = authority.to_lowercase();

            match authority.rfind(':') {
                None => output.push_str(&encode(&authority, false, true)),
                Some(colon) => {
                    output.push_str(&encode(&authority[..colon], false, true));
                    output.push_str(&authority[colon..]);
                }
            }
        }

        if !self.components.path.is_empty() {
            output.push_str(&encode(
                &Self::lower_drive_letter(&self.components.path),
                true,
                false,
            ));
        }

        if !self.components.query.is_empty() {
            output.push('?');
            output.push_str(&encode(&self.components.query, false, false));
        }

        if !self.components.fragment.is_empty() {
            output.push('#');

            if skip_encoding {
                output.push_str(&self.components.fragment);
            } else {
                output.push_str(&PercentEncoding::encode(
                    &self.components.fragment,
                    false,
                    false,
                ));
            }
        }

        output
    }

    fn lower_drive_letter(path: &str) -> Cow<'_, str> {
        let mut characters = path.chars();
        let (first, second, third) = (characters.next(), characters.next(), characters.next());

        if first == Some('/') && third == Some(':') {
            if let Some(letter) = second.filter(char::is_ascii_uppercase) {
                return Cow::Owned(format!("/{}:{}", letter.to_ascii_lowercase(), &path[3..]));
            }
        } else if second == Some(':')
            && let Some(letter) = first.filter(char::is_ascii_uppercase)
        {
            return Cow::Owned(format!("{}:{}", letter.to_ascii_lowercase(), &path[2..]));
        }

        Cow::Borrowed(path)
    }

    pub(super) fn file_system_path(&self) -> String {
        let mut characters = self.components.path.chars();
        let (first, second, third) = (characters.next(), characters.next(), characters.next());
        let value = if !self.components.authority.is_empty()
            && self.components.path.encode_utf16().nth(1).is_some()
            && self.components.scheme == Self::FILE
        {
            format!("//{}{}", self.components.authority, self.components.path)
        } else if first == Some('/')
            && second.is_some_and(|letter| letter.is_ascii_alphabetic())
            && third == Some(':')
        {
            let letter = second.unwrap_or_default().to_ascii_lowercase();

            format!("{letter}{}", &self.components.path[2..])
        } else {
            self.components.path.clone()
        };

        if cfg!(windows) {
            value.replace('/', "\\")
        } else {
            value
        }
    }
}
