pub(crate) struct HtmlEscape;

impl HtmlEscape {
    pub(crate) fn push(text: &str, out: &mut String) {
        for character in text.chars() {
            match character {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                '\'' => out.push_str("&#39;"),
                _ => out.push(character),
            }
        }
    }
}
