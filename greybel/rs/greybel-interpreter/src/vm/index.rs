use crate::value::Value;

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexResolver {
    len: usize,
}

impl IndexResolver {
    pub fn new(len: usize) -> Self {
        Self { len }
    }

    pub fn element(self, index: i32) -> Option<usize> {
        let len = i64::try_from(self.len).ok()?;
        let mut index = i64::from(index);

        if index < 0 {
            index += len;
        }

        (0..len)
            .contains(&index)
            .then(|| usize::try_from(index).ok())
            .flatten()
    }

    pub fn slice(self, start: &Value, end: &Value) -> std::ops::Range<usize> {
        let len = i64::try_from(self.len).unwrap_or(i64::MAX);
        let bound = |value: &Value, default: i64| -> usize {
            let mut index = if value.is_null() {
                default
            } else {
                i64::from(value.to_int())
            };

            if index < 0 {
                index += len;
            }

            usize::try_from(index.clamp(0, len)).unwrap_or_default()
        };
        let start = bound(start, 0);
        let end = bound(end, len);

        start..end.max(start)
    }
}
