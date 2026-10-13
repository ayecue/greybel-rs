use crate::fixed_point::FixedPoint;
use crate::outline::{Outline, PointKind};

/// Loads unhinted `glyf` outlines using FreeType-compatible composite transforms and rounding.
#[derive(Debug, Clone, Copy)]
pub struct TrueTypeLoader<'a> {
    locations: &'a [u8],
    glyphs: &'a [u8],
    long_offsets: bool,
}

#[derive(Debug, Clone, Copy)]
struct Reader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    const fn new(data: &'a [u8], offset: usize) -> Self {
        Self { data, offset }
    }

    fn u8(&mut self) -> Option<u8> {
        let value = *self.data.get(self.offset)?;

        self.offset += 1;

        Some(value)
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.data.get(self.offset..self.offset + 2)?;

        self.offset += 2;

        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn i16(&mut self) -> Option<i16> {
        self.u16().map(|value| value as i16)
    }

    fn skip(&mut self, count: usize) {
        self.offset += count;
    }
}

#[derive(Debug, Clone, Copy)]
struct Component {
    flags: u16,
    glyph: u16,
    arguments: (i64, i64),
    matrix: Option<[FixedPoint; 4]>,
}

impl Component {
    const ARGUMENTS_ARE_OFFSETS: u16 = 0x0002;
    const ARGUMENTS_ARE_WORDS: u16 = 0x0001;
    const MORE_COMPONENTS: u16 = 0x0020;
    const SCALE: u16 = 0x0008;
    const SCALED_OFFSET: u16 = 0x0800;
    const TWO_BY_TWO: u16 = 0x0080;
    const UNSCALED_OFFSET: u16 = 0x1000;
    const X_AND_Y_SCALE: u16 = 0x0040;

    fn read(reader: &mut Reader<'_>) -> Option<Self> {
        let flags = reader.u16()?;
        let glyph = reader.u16()?;
        let signed = flags & Self::ARGUMENTS_ARE_OFFSETS != 0;
        let arguments = match (flags & Self::ARGUMENTS_ARE_WORDS != 0, signed) {
            (true, true) => (i64::from(reader.i16()?), i64::from(reader.i16()?)),
            (true, false) => (i64::from(reader.u16()?), i64::from(reader.u16()?)),
            (false, true) => (i64::from(reader.u8()? as i8), i64::from(reader.u8()? as i8)),
            (false, false) => (i64::from(reader.u8()?), i64::from(reader.u8()?)),
        };
        let mut f2dot14 = || reader.i16().map(FixedPoint::from_f2dot14);
        let matrix = if flags & Self::SCALE != 0 {
            let scale = f2dot14()?;

            Some([scale, FixedPoint::ZERO, FixedPoint::ZERO, scale])
        } else if flags & Self::X_AND_Y_SCALE != 0 {
            let (x, y) = (f2dot14()?, f2dot14()?);

            Some([x, FixedPoint::ZERO, FixedPoint::ZERO, y])
        } else if flags & Self::TWO_BY_TWO != 0 {
            let (xx, yx, xy, yy) = (f2dot14()?, f2dot14()?, f2dot14()?, f2dot14()?);

            Some([xx, xy, yx, yy])
        } else {
            None
        };

        Some(Self {
            flags,
            glyph,
            arguments,
            matrix,
        })
    }

    fn offset(&self, scale: FixedPoint, base: &Outline, component: &Outline) -> (i64, i64) {
        if self.flags & Self::ARGUMENTS_ARE_OFFSETS == 0 {
            let anchor = usize::try_from(self.arguments.0).ok();
            let point = usize::try_from(self.arguments.1).ok();

            return match (
                anchor.and_then(|index| base.point(index)),
                point.and_then(|index| component.point(index)),
            ) {
                (Some(anchor), Some(point)) => (anchor.x - point.x, anchor.y - point.y),
                _ => (0, 0),
            };
        }

        let (mut x, mut y) = self.arguments;

        if let Some([xx, xy, yx, yy]) = self.matrix
            && self.flags & Self::SCALED_OFFSET != 0
            && self.flags & Self::UNSCALED_OFFSET == 0
        {
            x = xx.hypot(xy).scale(x);
            y = yy.hypot(yx).scale(y);
        }

        (scale.scale(x), scale.scale(y))
    }
}

impl<'a> TrueTypeLoader<'a> {
    const MAX_DEPTH: u8 = 16;

    #[must_use]
    pub fn new(locations: &'a [u8], glyphs: &'a [u8], long_offsets: bool) -> Self {
        Self {
            locations,
            glyphs,
            long_offsets,
        }
    }

    #[must_use]
    pub fn load(&self, glyph: u16, scale: FixedPoint) -> Outline {
        let mut outline = Outline::default();

        self.load_into(glyph, scale, 0, &mut outline);

        outline
    }

    fn range(&self, glyph: u16) -> Option<(usize, usize)> {
        let index = usize::from(glyph);
        let mut reader = if self.long_offsets {
            Reader::new(self.locations, index * 4)
        } else {
            Reader::new(self.locations, index * 2)
        };
        let mut location = || -> Option<usize> {
            if self.long_offsets {
                let high = u32::from(reader.u16()?);
                let low = u32::from(reader.u16()?);

                usize::try_from((high << 16) | low).ok()
            } else {
                Some(usize::from(reader.u16()?) * 2)
            }
        };
        let start = location()?;
        let end = location()?;

        (end > start && end <= self.glyphs.len()).then_some((start, end))
    }

    fn load_into(
        &self,
        glyph: u16,
        scale: FixedPoint,
        depth: u8,
        outline: &mut Outline,
    ) -> Option<()> {
        if depth > Self::MAX_DEPTH {
            return None;
        }

        let (start, end) = self.range(glyph)?;
        let data = &self.glyphs[start..end];
        let mut reader = Reader::new(data, 0);
        let contours = reader.i16()?;

        reader.skip(8);
        if contours >= 0 {
            return Self::simple(&mut reader, usize::try_from(contours).ok()?, scale, outline);
        }

        loop {
            let component = Component::read(&mut reader)?;
            let mut part = Outline::default();

            self.load_into(component.glyph, scale, depth + 1, &mut part);
            if let Some([xx, xy, yx, yy]) = component.matrix {
                part.transform(xx, xy, yx, yy);
            }

            let (x, y) = component.offset(scale, outline, &part);

            part.translate(x, y);
            outline.append(part);
            if component.flags & Component::MORE_COMPONENTS == 0 {
                return Some(());
            }
        }
    }

    fn simple(
        reader: &mut Reader<'_>,
        contours: usize,
        scale: FixedPoint,
        outline: &mut Outline,
    ) -> Option<()> {
        const ON_CURVE: u8 = 0x01;
        const X_SHORT: u8 = 0x02;
        const Y_SHORT: u8 = 0x04;
        const REPEAT: u8 = 0x08;
        const X_SAME_OR_POSITIVE: u8 = 0x10;
        const Y_SAME_OR_POSITIVE: u8 = 0x20;

        let mut ends = Vec::with_capacity(contours);

        for _ in 0..contours {
            ends.push(usize::from(reader.u16()?));
        }

        let count = ends.last().map_or(0, |end| end + 1);
        let instructions = usize::from(reader.u16()?);

        reader.skip(instructions);

        let mut flags = Vec::with_capacity(count);

        while flags.len() < count {
            let flag = reader.u8()?;

            flags.push(flag);
            if flag & REPEAT != 0 {
                for _ in 0..reader.u8()? {
                    flags.push(flag);
                }
            }
        }

        flags.truncate(count);

        let mut coordinate = |short: u8, same_or_positive: u8| -> Option<Vec<i64>> {
            let mut value = 0;
            let mut values = Vec::with_capacity(count);

            for &flag in &flags {
                if flag & short != 0 {
                    let delta = i64::from(reader.u8()?);

                    value += if flag & same_or_positive != 0 {
                        delta
                    } else {
                        -delta
                    };
                } else if flag & same_or_positive == 0 {
                    value += i64::from(reader.i16()?);
                }

                values.push(value);
            }

            Some(values)
        };
        let xs = coordinate(X_SHORT, X_SAME_OR_POSITIVE)?;
        let ys = coordinate(Y_SHORT, Y_SAME_OR_POSITIVE)?;
        let mut start = 0;

        for end in ends {
            for index in start..=end.min(count.saturating_sub(1)) {
                let kind = if flags[index] & ON_CURVE != 0 {
                    PointKind::OnCurve
                } else {
                    PointKind::Quadratic
                };

                outline.push_point(scale.scale(xs[index]), scale.scale(ys[index]), kind);
            }

            outline.close_contour();
            start = end + 1;
        }

        Some(())
    }
}
