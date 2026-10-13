use std::fmt;

use text_mesh_font::{
    FaceInfo, FontAsset, FontAtlas, FontSet, FontStyleSettings, Glyph, GlyphAdjustment,
    GlyphMetrics, GlyphRect, Kerning, KerningPair, SdfMaterial,
};

use super::RustSource;
use super::literal::Literal;

impl fmt::Display for RustSource<'_, FontSet> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let set = self.value;

        self.open(f, "::text_mesh_font::FontSet")?;
        self.slice(f, "fonts", &set.fonts)?;
        self.field(f, "primary", set.primary)?;
        self.slice(f, "global_fallbacks", &set.global_fallbacks)?;
        self.field(f, "missing_glyph", Literal::char(set.missing_glyph))?;
        self.field(f, "sprites", self.descend(&set.sprites))?;

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, FontAsset> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let font = self.value;

        self.open(f, "::text_mesh_font::FontAsset")?;
        self.field(f, "name", Literal::borrowed_str(&font.name))?;
        self.field(f, "face", self.descend(&font.face))?;
        self.field(f, "style", self.descend(&font.style))?;
        self.slice(f, "glyphs", &font.glyphs)?;
        self.slice(f, "kerning", &font.kerning)?;
        self.slice(f, "fallbacks", &font.fallbacks)?;
        match &font.material {
            Some(material) => {
                self.field(f, "material", format!("Some({})", self.descend(material)))?;
            }
            None => self.field(f, "material", "None")?,
        }

        match &font.atlas {
            Some(atlas) => self.field(f, "atlas", format!("Some({})", self.descend(atlas)))?,
            None => self.field(f, "atlas", "None")?,
        }

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, FaceInfo> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let face = self.value;

        self.open(f, "::text_mesh_font::FaceInfo")?;
        for (name, value) in [
            ("point_size", face.point_size),
            ("scale", face.scale),
            ("line_height", face.line_height),
            ("ascent_line", face.ascent_line),
            ("cap_line", face.cap_line),
            ("mean_line", face.mean_line),
            ("baseline", face.baseline),
            ("descent_line", face.descent_line),
            ("superscript_offset", face.superscript_offset),
            ("superscript_size", face.superscript_size),
            ("subscript_offset", face.subscript_offset),
            ("subscript_size", face.subscript_size),
            ("underline_offset", face.underline_offset),
            ("underline_thickness", face.underline_thickness),
            ("strikethrough_offset", face.strikethrough_offset),
            ("strikethrough_thickness", face.strikethrough_thickness),
            ("tab_width", face.tab_width),
        ] {
            self.field(f, name, Literal::float(value))?;
        }

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, FontStyleSettings> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let style = self.value;

        self.open(f, "::text_mesh_font::FontStyleSettings")?;
        self.field(f, "normal_style", Literal::float(style.normal_style))?;
        self.field(
            f,
            "normal_spacing_offset",
            Literal::float(style.normal_spacing_offset),
        )?;
        self.field(f, "bold_style", Literal::float(style.bold_style))?;
        self.field(f, "bold_spacing", Literal::float(style.bold_spacing))?;
        self.field(f, "italic_style", style.italic_style)?;
        self.field(f, "tab_size", style.tab_size)?;

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, SdfMaterial> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let material = self.value;

        self.open(f, "::text_mesh_font::SdfMaterial")?;
        for (name, value) in [
            ("gradient_scale", material.gradient_scale),
            ("face_dilate", material.face_dilate),
            ("outline_width", material.outline_width),
            ("outline_softness", material.outline_softness),
            ("weight_normal", material.weight_normal),
            ("weight_bold", material.weight_bold),
        ] {
            self.field(f, name, Literal::float(value))?;
        }

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, Glyph> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let glyph = self.value;

        write!(
            f,
            "::text_mesh_font::Glyph {{ character: {}, metrics: {}, rect: {} }}",
            Literal::char(glyph.character),
            self.descend(&glyph.metrics),
            self.descend(&glyph.rect)
        )
    }
}

impl fmt::Display for RustSource<'_, GlyphMetrics> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let metrics = self.value;

        write!(
            f,
            "::text_mesh_font::GlyphMetrics {{ width: {}, height: {}, bearing_x: {}, bearing_y: {}, advance: {} }}",
            Literal::float(metrics.width),
            Literal::float(metrics.height),
            Literal::float(metrics.bearing_x),
            Literal::float(metrics.bearing_y),
            Literal::float(metrics.advance)
        )
    }
}

impl fmt::Display for RustSource<'_, Kerning> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kerning = self.value;

        write!(
            f,
            "::text_mesh_font::Kerning {{ first: {}, second: {}, pair: {} }}",
            Literal::char(kerning.first),
            Literal::char(kerning.second),
            self.descend(&kerning.pair)
        )
    }
}

impl fmt::Display for RustSource<'_, KerningPair> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "::text_mesh_font::KerningPair {{ first: {}, second: {} }}",
            self.descend(&self.value.first),
            self.descend(&self.value.second)
        )
    }
}

impl fmt::Display for RustSource<'_, GlyphAdjustment> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let adjustment = self.value;

        write!(
            f,
            "::text_mesh_font::GlyphAdjustment {{ x_placement: {}, y_placement: {}, x_advance: {}, y_advance: {} }}",
            Literal::float(adjustment.x_placement),
            Literal::float(adjustment.y_placement),
            Literal::float(adjustment.x_advance),
            Literal::float(adjustment.y_advance)
        )
    }
}

impl fmt::Display for RustSource<'_, GlyphRect> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rect = self.value;

        write!(
            f,
            "::text_mesh_font::GlyphRect {{ x: {}, y: {}, width: {}, height: {}, atlas: {} }}",
            rect.x, rect.y, rect.width, rect.height, rect.atlas
        )
    }
}

impl fmt::Display for RustSource<'_, FontAtlas> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let atlas = self.value;

        write!(
            f,
            "::text_mesh_font::FontAtlas {{ width: {}, height: {}, padding: {}, render_mode: ::text_mesh_font::RenderMode::{:?}, count: {} }}",
            atlas.width, atlas.height, atlas.padding, atlas.render_mode, atlas.count
        )
    }
}
