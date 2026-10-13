//! Loads TextMesh Pro setups consumed and emitted by the Unity reference project.

use std::borrow::Cow;

use serde::Deserialize;
use text_mesh_core::{Alignment, Color, TextAssetTable, TextStyle};
use text_mesh_font::{
    FaceInfo, FontAsset, FontAtlas, FontSet, FontStyleSettings, Glyph, GlyphAdjustment,
    GlyphMetrics, GlyphRect, Kerning, KerningPair, RenderMode, SdfMaterial, SpriteAsset,
    SpriteCharacter, SpriteGlyph, SpriteTexture,
};
use text_mesh_layout::{Margin, TerminalFrame, TextSettings, VerticalAlignment};

/// Setup read error: invalid setup JSON or missing required data.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("the setup is not JSON of a setup: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0} has no metrics: the setup gives only what to bake it from")]
    MissingMetrics(String),
    #[error("{font} uses render mode {mode}, which TMP does not have")]
    RenderMode { font: String, mode: u32 },
    #[error("the terminal's font {0} is not one of the setup's fonts")]
    Terminal(String),
    #[error("the setup has no {0}")]
    Missing(&'static str),
}

/// TextMesh Pro setup, as the reference project reads and exports it
/// (`Assets/Reference/Runtime/Setup.cs`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    text: TextData,
    frame: Option<FrameData>,
    theme: Option<ThemeData>,
    sprites: SpriteData,
    terminal: String,
    #[serde(default)]
    global_fallbacks: Vec<String>,
    missing_glyph_character: u32,
    fonts: Vec<FontData>,
    #[serde(default)]
    bundled: Vec<FontData>,
    text_assets: Option<TextAssetsData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextData {
    font_size: f32,
    #[serde(rename = "HorizontalAlignment")]
    horizontal_alignment: u32,
    #[serde(rename = "VerticalAlignment")]
    vertical_alignment: u32,
    character_spacing: f32,
    word_spacing: f32,
    line_spacing: f32,
    line_spacing_max: f32,
    paragraph_spacing: f32,
    enable_word_wrapping: u8,
    word_wrapping_ratios: f32,
    enable_kerning: u8,
    margin: [f32; 4],
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameData {
    padding: f32,
    item_gap: f32,
    item_min_height: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct ThemeData {
    text: ColorData,
    background: ColorData,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct ColorData {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FaceData {
    point_size: f32,
    scale: f32,
    line_height: f32,
    ascent_line: f32,
    cap_line: f32,
    mean_line: f32,
    baseline: f32,
    descent_line: f32,
    superscript_offset: f32,
    superscript_size: f32,
    subscript_offset: f32,
    subscript_size: f32,
    underline_offset: f32,
    underline_thickness: f32,
    strikethrough_offset: f32,
    strikethrough_thickness: f32,
    tab_width: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PropertiesData {
    normal_style: f32,
    normal_spacing_offset: f32,
    bold_style: f32,
    bold_spacing: f32,
    italic_style: u8,
    tab_size: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterData {
    unicode: u32,
    advance: f32,
    bearing_x: f32,
    bearing_y: f32,
    width: f32,
    height: f32,
    x: i32,
    y: i32,
    rect_width: i32,
    rect_height: i32,
    atlas: u8,
}

impl CharacterData {
    /// Assigns empty rectangles to space-like glyphs that Grey Hack's legacy Font Asset Creator stored outside the atlas;
    /// TMP does not draw them.
    fn rect(&self) -> GlyphRect {
        let field = |value: i32| u16::try_from(value).ok();

        match (
            field(self.x),
            field(self.y),
            field(self.rect_width),
            field(self.rect_height),
        ) {
            (Some(x), Some(y), Some(width), Some(height)) => GlyphRect {
                x,
                y,
                width,
                height,
                atlas: self.atlas,
            },
            _ => GlyphRect::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustmentData {
    x_placement: f32,
    y_placement: f32,
    x_advance: f32,
    y_advance: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KerningData {
    first: u32,
    second: u32,
    first_adjustment: AdjustmentData,
    second_adjustment: AdjustmentData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FontData {
    name: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    padding: u16,
    #[serde(default)]
    atlas_width: u16,
    #[serde(default)]
    atlas_height: u16,
    #[serde(default)]
    render_mode: u32,
    #[serde(default)]
    atlases: Vec<String>,
    face: Option<FaceData>,
    properties: Option<PropertiesData>,
    #[serde(default)]
    floats: Vec<FloatData>,
    #[serde(default)]
    fallbacks: Vec<String>,
    #[serde(default)]
    characters: Vec<CharacterData>,
    #[serde(default)]
    kerning: Vec<KerningData>,
}

#[derive(Debug, Clone, Deserialize)]
struct FloatData {
    name: String,
    value: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpriteGlyphData {
    x: u16,
    y: u16,
    rect_width: u16,
    rect_height: u16,
    advance: f32,
    bearing_x: f32,
    bearing_y: f32,
    width: f32,
    height: f32,
    scale: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpriteCharacterData {
    name: String,
    unicode: u32,
    glyph: usize,
    scale: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpriteData {
    name: String,
    sheet: String,
    sheet_width: u16,
    sheet_height: u16,
    face: Option<FaceData>,
    glyphs: Vec<SpriteGlyphData>,
    characters: Vec<SpriteCharacterData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextAssetsData {
    fonts: Vec<String>,
    materials: Vec<String>,
    sprites: Vec<String>,
    styles: Vec<StyleData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StyleData {
    name: String,
    open_markup: String,
    close_markup: String,
}

impl Setup {
    /// Parses setup JSON from a file.
    ///
    /// # Errors
    /// Returns [`SetupError::Json`] if the file is not a valid setup.
    pub fn read(json: &str) -> Result<Self, SetupError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Loads font metrics in setup order, starting with the terminal font.
    ///
    /// # Errors
    /// Returns [`SetupError::MissingMetrics`] for missing font metrics, [`SetupError::RenderMode`] for an unknown TMP
    /// render mode, or [`SetupError::Terminal`] if the terminal font is absent.
    pub fn font_set(&self) -> Result<FontSet, SetupError> {
        let all: Vec<&FontData> = self.fonts.iter().chain(&self.bundled).collect();
        let index = |name: &str| all.iter().position(|font| font.name == name);
        let fonts = all
            .iter()
            .map(|font| font.asset(&index))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(FontSet {
            fonts: Cow::Owned(fonts),
            primary: index(&self.terminal)
                .ok_or_else(|| SetupError::Terminal(self.terminal.clone()))?,
            global_fallbacks: Cow::Owned(
                self.global_fallbacks
                    .iter()
                    .filter_map(|name| index(name))
                    .collect(),
            ),
            missing_glyph: char::from_u32(self.missing_glyph_character).unwrap_or('?'),
            sprites: self.sprites.asset(),
        })
    }

    /// Text settings of a terminal line.
    #[must_use]
    pub fn text_settings(&self) -> TextSettings {
        self.text.settings()
    }

    /// Terminal's frame around its lines.
    ///
    /// # Errors
    /// [`SetupError::Missing`] when the setup has none.
    pub fn frame(&self) -> Result<TerminalFrame, SetupError> {
        let frame = self.frame.ok_or(SetupError::Missing("frame"))?;

        Ok(TerminalFrame {
            padding: frame.padding,
            item_gap: frame.item_gap,
            item_min_height: frame.item_min_height,
        })
    }

    /// Terminal's text and background colors.
    ///
    /// # Errors
    /// [`SetupError::Missing`] when the setup has no theme.
    pub fn colors(&self) -> Result<(Color, Color), SetupError> {
        let theme = self.theme.ok_or(SetupError::Missing("theme"))?;

        Ok((theme.text.color(), theme.background.color()))
    }

    /// Exported named rich-text lookup data.
    ///
    /// # Errors
    /// [`SetupError::Missing`] when the setup was not exported.
    pub fn text_assets(&self) -> Result<TextAssetTable, SetupError> {
        let assets = self
            .text_assets
            .as_ref()
            .ok_or(SetupError::Missing("textAssets"))?;
        let names = |names: &[String]| {
            Cow::Owned(names.iter().map(|name| Cow::Owned(name.clone())).collect())
        };

        Ok(TextAssetTable {
            fonts: names(&assets.fonts),
            materials: names(&assets.materials),
            sprites: names(&assets.sprites),
            styles: Cow::Owned(
                assets
                    .styles
                    .iter()
                    .map(|style| TextStyle {
                        name: Cow::Owned(style.name.clone()),
                        open_markup: Cow::Owned(style.open_markup.clone()),
                        close_markup: Cow::Owned(style.close_markup.clone()),
                    })
                    .collect(),
            ),
        })
    }

    /// Font atlas files (`atlases/` next to the setup): font, atlas index, and file name.
    #[must_use]
    pub fn atlas_files(&self) -> Vec<(String, u8, String)> {
        self.fonts
            .iter()
            .chain(&self.bundled)
            .flat_map(|font| {
                font.atlases
                    .iter()
                    .zip(0u8..)
                    .map(|(file, index)| (font.name.clone(), index, file.clone()))
            })
            .collect()
    }

    /// Font names and source files (`fonts/` next to the setup), when specified.
    #[must_use]
    pub fn font_sources(&self) -> Vec<(&str, &str)> {
        self.fonts
            .iter()
            .chain(&self.bundled)
            .filter(|font| !font.source.is_empty())
            .map(|font| (font.name.as_str(), font.source.as_str()))
            .collect()
    }

    /// Sprite asset's sheet (`sprites/` next to the setup).
    #[must_use]
    pub fn sprite_sheet(&self) -> &str {
        &self.sprites.sheet
    }
}

impl TextData {
    fn settings(&self) -> TextSettings {
        TextSettings {
            font_size: self.font_size,
            alignment: match self.horizontal_alignment {
                2 => Alignment::Center,
                4 => Alignment::Right,
                8 => Alignment::Justified,
                16 => Alignment::Flush,
                _ => Alignment::Left,
            },
            vertical_alignment: match self.vertical_alignment {
                512 => VerticalAlignment::Middle,
                1024 => VerticalAlignment::Bottom,
                2048 => VerticalAlignment::Baseline,
                4096 => VerticalAlignment::Geometry,
                8192 => VerticalAlignment::Capline,
                _ => VerticalAlignment::Top,
            },
            character_spacing: self.character_spacing,
            word_spacing: self.word_spacing,
            line_spacing: self.line_spacing,
            line_spacing_max: self.line_spacing_max,
            paragraph_spacing: self.paragraph_spacing,
            word_wrapping: self.enable_word_wrapping != 0,
            word_wrapping_ratios: self.word_wrapping_ratios,
            kerning: self.enable_kerning != 0,
            margin: Margin {
                left: self.margin[0],
                top: self.margin[1],
                right: self.margin[2],
                bottom: self.margin[3],
            },
        }
    }
}

impl ColorData {
    fn color(self) -> Color {
        Color::new(self.r, self.g, self.b, self.a)
    }
}

impl FaceData {
    fn face(&self) -> FaceInfo {
        FaceInfo {
            point_size: self.point_size,
            scale: self.scale,
            line_height: self.line_height,
            ascent_line: self.ascent_line,
            cap_line: self.cap_line,
            mean_line: self.mean_line,
            baseline: self.baseline,
            descent_line: self.descent_line,
            superscript_offset: self.superscript_offset,
            superscript_size: self.superscript_size,
            subscript_offset: self.subscript_offset,
            subscript_size: self.subscript_size,
            underline_offset: self.underline_offset,
            underline_thickness: self.underline_thickness,
            strikethrough_offset: self.strikethrough_offset,
            strikethrough_thickness: self.strikethrough_thickness,
            tab_width: self.tab_width,
        }
    }
}

impl AdjustmentData {
    fn adjustment(&self) -> GlyphAdjustment {
        GlyphAdjustment {
            x_placement: self.x_placement,
            y_placement: self.y_placement,
            x_advance: self.x_advance,
            y_advance: self.y_advance,
        }
    }
}

impl FontData {
    fn asset(&self, index: &impl Fn(&str) -> Option<usize>) -> Result<FontAsset, SetupError> {
        let missing_metrics = || SetupError::MissingMetrics(self.name.clone());
        let face = self.face.as_ref().ok_or_else(missing_metrics)?;
        let properties = self.properties.as_ref().ok_or_else(missing_metrics)?;

        if self.atlases.is_empty() {
            return Err(missing_metrics());
        }

        let glyphs = self.characters.iter().filter_map(|character| {
            Some(Glyph {
                character: char::from_u32(character.unicode)?,
                metrics: GlyphMetrics {
                    width: character.width,
                    height: character.height,
                    bearing_x: character.bearing_x,
                    bearing_y: character.bearing_y,
                    advance: character.advance,
                },
                rect: character.rect(),
            })
        });
        let atlas = FontAtlas {
            width: self.atlas_width,
            height: self.atlas_height,
            padding: self.padding,
            render_mode: RenderMode::from_unity(self.render_mode).ok_or_else(|| {
                SetupError::RenderMode {
                    font: self.name.clone(),
                    mode: self.render_mode,
                }
            })?,
            count: u8::try_from(self.atlases.len()).unwrap_or(u8::MAX),
        };
        let kerning = self.kerning.iter().filter_map(|pair| {
            Some(Kerning {
                first: char::from_u32(pair.first)?,
                second: char::from_u32(pair.second)?,
                pair: KerningPair {
                    first: pair.first_adjustment.adjustment(),
                    second: pair.second_adjustment.adjustment(),
                },
            })
        });

        Ok(FontAsset {
            name: Cow::Owned(self.name.clone()),
            face: face.face(),
            style: FontStyleSettings {
                normal_style: properties.normal_style,
                normal_spacing_offset: properties.normal_spacing_offset,
                bold_style: properties.bold_style,
                bold_spacing: properties.bold_spacing,
                italic_style: properties.italic_style,
                tab_size: properties.tab_size,
            },
            fallbacks: Cow::Owned(
                self.fallbacks
                    .iter()
                    .filter_map(|name| index(name))
                    .collect(),
            ),
            material: self.material(),
            atlas: Some(atlas),
            ..FontAsset::default()
        }
        .with_glyphs(glyphs)
        .with_kerning(kerning))
    }

    fn material(&self) -> Option<SdfMaterial> {
        let float = |name: &str| {
            self.floats
                .iter()
                .find(|float| float.name == name)
                .map(|float| float.value)
        };

        Some(SdfMaterial {
            gradient_scale: float("_GradientScale")?,
            face_dilate: float("_FaceDilate").unwrap_or(0.0),
            outline_width: float("_OutlineWidth").unwrap_or(0.0),
            outline_softness: float("_OutlineSoftness").unwrap_or(0.0),
            weight_normal: float("_WeightNormal").unwrap_or(0.0),
            weight_bold: float("_WeightBold").unwrap_or(0.0),
        })
    }
}

impl SpriteData {
    fn asset(&self) -> SpriteAsset {
        SpriteAsset {
            name: Cow::Owned(self.name.clone()),
            face: self.face.as_ref().map(FaceData::face).unwrap_or_default(),
            texture: Some(SpriteTexture {
                width: self.sheet_width,
                height: self.sheet_height,
            }),
            glyphs: Cow::Owned(
                self.glyphs
                    .iter()
                    .map(|glyph| SpriteGlyph {
                        metrics: GlyphMetrics {
                            width: glyph.width,
                            height: glyph.height,
                            bearing_x: glyph.bearing_x,
                            bearing_y: glyph.bearing_y,
                            advance: glyph.advance,
                        },
                        rect: GlyphRect {
                            x: glyph.x,
                            y: glyph.y,
                            width: glyph.rect_width,
                            height: glyph.rect_height,
                            atlas: 0,
                        },
                        scale: glyph.scale,
                    })
                    .collect(),
            ),
            characters: Cow::Owned(
                self.characters
                    .iter()
                    .map(|character| SpriteCharacter {
                        name: Cow::Owned(character.name.clone()),
                        unicode: character.unicode,
                        glyph: character.glyph,
                        scale: character.scale,
                    })
                    .collect(),
            ),
        }
    }
}
