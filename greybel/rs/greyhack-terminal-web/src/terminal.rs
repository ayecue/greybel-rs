use duplex_core::Bytes;
use greybel_io::{Output, TextScreen};
use greyhack_text_mesh_assets::{AtlasFile, TERMINAL_TEXT, TERMINAL_THEME};
use text_mesh_core::LineSplitRule;
use text_mesh_webgl::{MeshMaterial, Projection, SamplePattern, SdfUniforms};

use crate::protocol::terminal::{
    Atlas, Color, DistanceField, Frame, Image, Material, Mesh, Scene, TextureFailure, TextureSet,
    Vector2, Vector4, Viewport,
};
use crate::view::TerminalView;

/// Worker terminal state containing a `TextScreen` and `TerminalView`. Draw scenes use Grey Hack's multisampling
/// and anisotropic filtering settings.
#[derive(Debug, Clone)]
pub struct Terminal {
    screen: TextScreen,
    view: TerminalView,
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

impl Terminal {
    const ANISOTROPY: u8 = 16;
    const SAMPLE_COUNT: u8 = 2;

    /// Creates an empty terminal whose initial viewport is 1 × 1.
    #[must_use]
    pub fn new() -> Self {
        Self {
            screen: TextScreen::default(),
            view: TerminalView::new(1.0, 1.0),
        }
    }

    #[must_use]
    pub const fn view(&self) -> &TerminalView {
        &self.view
    }

    /// Applies program `output` to the screen and refreshes the view text.
    pub fn show(&mut self, output: &[Output]) {
        self.screen.apply(output);
        self.view.set_text(&self.screen.text());
    }

    /// Splits long lines by `split_rule` and relayouts the output.
    pub fn set_split_rule(&mut self, split_rule: LineSplitRule) {
        self.view.set_split_rule(split_rule);
        self.view.set_text(&self.screen.text());
    }

    /// Scrolls by `delta` CSS pixels, with positive values moving down.
    pub fn scroll_by(&mut self, delta: f32) {
        self.view.scroll_by(delta);
    }

    /// Builds the scene for `viewport`, relayouts after width changes, and applies the game's sampling and filtering.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "Viewport dimensions are clamped positive and converted to whole physical pixels"
    )]
    pub fn draw(&mut self, viewport: &Viewport) -> Scene {
        let (width, height) = (viewport.width.max(1.0), viewport.height.max(1.0));

        self.view.resize(width, height);

        let projection = Projection {
            width,
            height,
            pixel_ratio: viewport.pixel_ratio,
        };
        let [screen_width, screen_height] = projection.screen();
        let background = TERMINAL_THEME.background;
        let frame = Frame {
            width: screen_width.max(1.0) as u32,
            height: screen_height.max(1.0) as u32,
            projection: projection.matrix().to_vec(),
            samples: SamplePattern::offsets(Self::SAMPLE_COUNT)
                .iter()
                .map(|&[x, y]| Vector2 {
                    x: x * viewport.pixel_ratio,
                    y: -y * viewport.pixel_ratio,
                })
                .collect(),
            anisotropy: Self::ANISOTROPY,
            background: Color {
                r: background.r,
                g: background.g,
                b: background.b,
                a: background.a,
            },
        };
        let meshes = self
            .view
            .meshes()
            .into_iter()
            .filter(|mesh| mesh.vertex_count() > 0)
            .map(|mesh| Mesh {
                material: Self::material(mesh.material),
                vertices: Bytes(mesh.vertex_data()),
            })
            .collect();

        Scene { frame, meshes }
    }

    /// Returns font atlases in setup order, then the game's sprite sheet.
    ///
    /// # Errors
    ///
    /// Fails with `TextureFailure::MissingAtlas` if a required font atlas is absent.
    pub fn textures() -> Result<TextureSet, TextureFailure> {
        let mut atlases = Vec::new();

        for (font, asset) in TERMINAL_TEXT.font_set.fonts.iter().enumerate() {
            for index in 0..asset.atlas.map_or(0, |atlas| atlas.count) {
                let file = AtlasFile::find(&asset.name, index).ok_or_else(|| {
                    TextureFailure::MissingAtlas {
                        font: asset.name.to_string(),
                        index,
                    }
                })?;

                atlases.push(Atlas {
                    font: u32::try_from(font).expect("the font index fits u32"),
                    index,
                    width: u32::from(file.width),
                    height: u32::from(file.height),
                    alpha: Bytes::from(file.alpha),
                });
            }
        }

        let sheet = TERMINAL_TEXT.sprite_sheet;

        Ok(TextureSet {
            atlases,
            sprite_sheet: Image {
                width: u32::from(sheet.width),
                height: u32::from(sheet.height),
                rgba: Bytes::from(sheet.rgba),
            },
        })
    }

    fn material(material: MeshMaterial) -> Material {
        let Some((font, atlas)) = material.font() else {
            return Material::Sprites;
        };

        Material::Font {
            font: u32::try_from(font).expect("the font index fits u32"),
            atlas,
            uniforms: SdfUniforms::of(&TERMINAL_TEXT.font_set, material).map(|uniforms| {
                let vector2 = |[x, y]: [f32; 2]| Vector2 { x, y };
                let vector4 = |[x, y, z, w]: [f32; 4]| Vector4 { x, y, z, w };

                DistanceField {
                    face_color: vector4(uniforms.face_color),
                    outline_color: vector4(uniforms.outline_color),
                    gradient_scale: uniforms.gradient_scale,
                    sharpness: uniforms.sharpness,
                    weight_normal: uniforms.weight_normal,
                    weight_bold: uniforms.weight_bold,
                    face_dilate: uniforms.face_dilate,
                    scale_ratio_a: uniforms.scale_ratio_a,
                    outline_width: uniforms.outline_width,
                    outline_softness: uniforms.outline_softness,
                    scale: vector2(uniforms.scale),
                    vertex_offset: vector2(uniforms.vertex_offset),
                }
            }),
        }
    }
}
