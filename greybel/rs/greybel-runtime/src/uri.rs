use greybel_documents::Uri;

/// Helpers for canonicalizing editor URI strings.
pub(crate) trait EditorUri {
    /// Normalizes a URI string, returning the original text when parsing fails.
    fn canonical(&self) -> String;
}

impl EditorUri for str {
    fn canonical(&self) -> String {
        Uri::parse(self).map_or_else(|_| self.to_owned(), |uri| uri.to_string())
    }
}
