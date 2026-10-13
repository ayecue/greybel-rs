const DIGIT_ZERO: u16 = b'0' as u16;
const DIGIT_NINE: u16 = b'9' as u16;
const PERIOD: u16 = b'.' as u16;
const COMMA: u16 = b',' as u16;
const SPACE: u16 = b' ' as u16;

pub(crate) struct TagNumber;

impl TagNumber {
    pub(crate) const INVALID: f32 = -32768.0;
    const PARAMETER_CAPACITY: usize = 16;

    pub(crate) fn parse(text: &[u16], start: usize, length: usize) -> f32 {
        Self::parse_until(text, start, length).0
    }

    pub(crate) fn parameters(text: &[u16], start: usize, length: usize) -> Vec<f32> {
        let end = start + length;
        let mut start = start;
        let mut last = start;
        let mut parameters = Vec::new();

        while last < end && parameters.len() < Self::PARAMETER_CAPACITY {
            let (value, until) = Self::parse_until(text, start, end - start);

            parameters.push(value);
            last = until;
            start = last + 1;
        }

        parameters
    }

    #[allow(clippy::cast_precision_loss)]
    fn parse_until(text: &[u16], start: usize, length: usize) -> (f32, usize) {
        if start == 0 {
            return (Self::INVALID, 0);
        }

        let end = start + length;
        let unit = |index: usize| text.get(index).copied().unwrap_or(0);
        let mut index = start;
        let mut sign: i64 = 1;

        if unit(index) == u16::from(b'+') {
            index += 1;
        } else if unit(index) == u16::from(b'-') {
            sign = -1;
            index += 1;
        }

        let mut in_integer_part = true;
        let mut fraction = 0.0_f32;
        let mut value = 0.0_f32;

        while index < end {
            let current = unit(index);

            if !(DIGIT_ZERO..=DIGIT_NINE).contains(&current) {
                match current {
                    PERIOD => {}
                    COMMA => {
                        let last = if index + 1 < end && unit(index + 1) == SPACE {
                            index + 1
                        } else {
                            index
                        };

                        return (Self::reject_overflow(value), last);
                    }
                    _ => {
                        index += 1;
                        continue;
                    }
                }
            }

            if current == PERIOD {
                in_integer_part = false;
                fraction = 0.1;
            } else if in_integer_part {
                value = value * 10.0 + (i64::from(current - DIGIT_ZERO) * sign) as f32;
            } else {
                value += f32::from(current - DIGIT_ZERO) * fraction * sign as f32;
                fraction *= 0.1;
            }

            index += 1;
        }

        (Self::reject_overflow(value), end)
    }

    fn reject_overflow(value: f32) -> f32 {
        if value > 32767.0 {
            Self::INVALID
        } else {
            value
        }
    }
}
