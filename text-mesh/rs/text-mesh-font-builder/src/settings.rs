use text_mesh_font::{FontStyleSettings, RenderMode, SdfMaterial};

/// Configures font sampling and new TMP asset and material defaults like Font Asset Creator.
#[derive(Debug, Clone, PartialEq)]
pub struct FontBuildSettings {
    pub name: String,
    pub point_size: u16,
    pub padding: u16,
    pub atlas_width: u16,
    pub atlas_height: u16,
    pub render_mode: RenderMode,
    /// Order TMP adds characters in, which decides where the packer places each glyph.
    pub characters: Vec<char>,
    pub kerning: bool,
    pub style: FontStyleSettings,
    /// Replaces TMP's default material for new SDF assets; bitmap render modes ignore this override.
    pub material: Option<SdfMaterial>,
}

impl FontBuildSettings {
    pub const DEFAULT_ATLAS_SIZE: u16 = 1024;
    pub const DEFAULT_PADDING: u16 = 9;
    /// Sets sampling size, padding, render mode, and atlas size using `TMP_FontAsset.CreateFontAsset(Font)` defaults.
    pub const DEFAULT_POINT_SIZE: u16 = 90;
    pub const DEFAULT_RENDER_MODE: RenderMode = RenderMode::Sdfaa;
    /// Default field values from `TMP_FontAsset` initialization.
    pub const DEFAULT_STYLE: FontStyleSettings = FontStyleSettings {
        normal_style: 0.0,
        normal_spacing_offset: 0.0,
        bold_style: 0.75,
        bold_spacing: 7.0,
        italic_style: 35,
        tab_size: 10,
    };

    #[must_use]
    pub fn new(name: impl Into<String>, characters: impl IntoIterator<Item = char>) -> Self {
        Self {
            name: name.into(),
            point_size: Self::DEFAULT_POINT_SIZE,
            padding: Self::DEFAULT_PADDING,
            atlas_width: Self::DEFAULT_ATLAS_SIZE,
            atlas_height: Self::DEFAULT_ATLAS_SIZE,
            render_mode: Self::DEFAULT_RENDER_MODE,
            characters: characters.into_iter().collect(),
            kerning: false,
            style: Self::DEFAULT_STYLE,
            material: None,
        }
    }

    /// Font Asset Creator's "ASCII" character set: space to tilde.
    #[must_use]
    pub fn ascii() -> Vec<char> {
        (' '..='~').collect()
    }

    #[must_use]
    pub const fn with_sampling_size(mut self, point_size: u16, padding: u16) -> Self {
        self.point_size = point_size;
        self.padding = padding;

        self
    }

    #[must_use]
    pub const fn with_atlas(mut self, width: u16, height: u16, render_mode: RenderMode) -> Self {
        self.atlas_width = width;
        self.atlas_height = height;
        self.render_mode = render_mode;

        self
    }

    #[must_use]
    pub const fn with_kerning(mut self, kerning: bool) -> Self {
        self.kerning = kerning;

        self
    }

    #[must_use]
    pub const fn with_style(mut self, style: FontStyleSettings) -> Self {
        self.style = style;

        self
    }

    #[must_use]
    pub const fn with_material(mut self, material: SdfMaterial) -> Self {
        self.material = Some(material);

        self
    }

    /// Applies `TMP_FontAsset.CreateFontAsset` settings to a new SDF material and leaves other shader defaults unchanged.
    #[must_use]
    pub fn default_material(&self) -> SdfMaterial {
        SdfMaterial {
            gradient_scale: f32::from(self.padding) + 1.0,
            face_dilate: 0.0,
            outline_width: 0.0,
            outline_softness: 0.0,
            weight_normal: self.style.normal_style,
            weight_bold: self.style.bold_style,
        }
    }

    #[must_use]
    pub fn material(&self) -> Option<SdfMaterial> {
        self.render_mode
            .is_distance_field()
            .then(|| self.material.unwrap_or_else(|| self.default_material()))
    }
}
