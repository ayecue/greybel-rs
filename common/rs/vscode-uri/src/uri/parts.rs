pub(super) struct RawUriComponentContainer<'a> {
    pub scheme: &'a str,
    pub authority: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub fragment: &'a str,
}

impl<'a> RawUriComponentContainer<'a> {
    const LINE_TERMINATORS: [char; 4] = ['\n', '\r', '\u{2028}', '\u{2029}'];

    pub(super) fn split(value: &'a str) -> Self {
        let mut remainder = value;
        let mut scheme = "";

        if let Some(index) = remainder.find([':', '/', '?', '#'])
            && index > 0
            && remainder.as_bytes()[index] == b':'
        {
            scheme = &remainder[..index];
            remainder = &remainder[index + 1..];
        }

        let mut authority = "";

        if let Some(after) = remainder.strip_prefix("//") {
            let end = after.find(['/', '?', '#']).unwrap_or(after.len());

            authority = &after[..end];
            remainder = &after[end..];
        }

        let end = remainder.find(['?', '#']).unwrap_or(remainder.len());
        let path = &remainder[..end];

        remainder = &remainder[end..];

        let mut query = "";

        if let Some(after) = remainder.strip_prefix('?') {
            let end = after.find('#').unwrap_or(after.len());

            query = &after[..end];
            remainder = &after[end..];
        }

        let mut fragment = "";

        if let Some(after) = remainder.strip_prefix('#') {
            let end = after.find(Self::LINE_TERMINATORS).unwrap_or(after.len());

            fragment = &after[..end];
        }

        Self {
            scheme,
            authority,
            path,
            query,
            fragment,
        }
    }
}
