mod auto_compile;
mod folders;
mod header;

pub(crate) use auto_compile::AUTO_COMPILE;
pub(crate) use folders::FOLDERS;
pub(crate) use header::HEADER;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ScriptTemplate(&'static str);

impl ScriptTemplate {
    pub(crate) fn render(&self, values: &[(&str, &str)]) -> String {
        let mut code = String::with_capacity(self.0.len());
        let mut remainder = self.0;

        while let Some(start) = remainder.find('$') {
            code.push_str(&remainder[..start]);

            let name_length = remainder[start + 1..]
                .find(|unit: char| !(unit.is_ascii_uppercase() || unit == '_'))
                .unwrap_or(remainder.len() - start - 1);
            let name = &remainder[start + 1..start + 1 + name_length];

            match values.iter().find(|(key, _)| *key == name) {
                Some((_, value)) => code.push_str(value),
                None => code.push_str(&remainder[start..=start + name_length]),
            }

            remainder = &remainder[start + 1 + name_length..];
        }

        code.push_str(remainder);

        Self::compact(&code)
    }

    fn compact(code: &str) -> String {
        code.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(";")
    }
}

pub(crate) struct GreyScriptText;

impl GreyScriptText {
    pub(crate) fn quote(text: &str) -> String {
        format!("\"{}\"", text.replace('"', "\"\""))
    }

    pub(crate) fn list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
        items
            .into_iter()
            .map(Self::quote)
            .collect::<Vec<_>>()
            .join(",")
    }
}
