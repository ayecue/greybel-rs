//! Meshes TMP drew every test case with, and WebGL mesh differences.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use text_mesh_core::{Color, TextAssetTable, TextComponent};
use text_mesh_font::FontSet;
use text_mesh_layout::{
    LayoutGeometry, TerminalFrame, TerminalItem, TerminalLayout, TextLayout, TextSettings,
};
use text_mesh_unity::{AssetRecord, Reference, ReferenceError, read_fingerprints};
use text_mesh_webgl::{Mesh, MeshBuilder, MeshMaterial, SpriteSheet};

use crate::TestSetup;
use crate::reference::{Captures, Difference, Differences, WIDTH};

const POSITION_TOLERANCE: f32 = 0.01;
const UV_TOLERANCE: f32 = 1e-5;
const SCALE_TOLERANCE: f32 = 1e-4;

/// Captured meshes in a fixtures folder.
const MESHES: &str = "tmp-meshes.json";
/// Fingerprints of the font and sprite assets the meshes were captured from, in a fixtures folder.
const ASSET_FINGERPRINTS: &str = "asset-fingerprints.json";

#[derive(Deserialize)]
pub struct MeshCase {
    pub source: String,
    pub image: String,
    pub height: u32,
    items: Vec<MeshItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MeshItem {
    lossy_scale: f32,
    meshes: Vec<MeshRecord>,
}

#[derive(Deserialize)]
struct MeshRecord {
    material: String,
    positions: Vec<f32>,
    uvs: Vec<f32>,
    scales: Vec<f32>,
    colors: Vec<i64>,
}

/// Terminal's setup apart from its fonts: what its lines are laid out and colored with.
#[derive(Clone, Copy)]
pub struct Terminal<'a> {
    pub assets: &'a TextAssetTable,
    pub settings: &'a TextSettings,
    pub component: TextComponent<'a>,
    pub frame: TerminalFrame,
    pub text: Color,
}

impl TestSetup {
    #[must_use]
    pub const fn terminal(&self) -> Terminal<'_> {
        Terminal {
            assets: &self.assets,
            settings: &self.settings,
            component: self.component(),
            frame: self.frame,
            text: self.text,
        }
    }
}

impl Terminal<'_> {
    /// `source` lines laid out with `fonts`, including geometry.
    #[must_use]
    pub fn lay_out(
        &self,
        fonts: &FontSet,
        source: &str,
        width: f32,
    ) -> Vec<(TerminalItem, LayoutGeometry)> {
        let text = TextLayout::new(fonts, self.assets, self.settings);

        TerminalLayout::new(text, self.component, self.frame).layout_with_geometry(source, width)
    }

    /// Line-local meshes; comparison applies the offset.
    #[must_use]
    pub fn build(
        &self,
        fonts: &FontSet,
        sprites: &SpriteSheet,
        source: &str,
        width: f32,
    ) -> Vec<Vec<Mesh>> {
        let builder = MeshBuilder::new(fonts, sprites, self.text);

        self.lay_out(fonts, source, width)
            .iter()
            .map(|(item, geometry)| builder.build(&item.layout, geometry, [0.0, 0.0]))
            .collect()
    }
}

/// Meshes TMP drew every test case with through one terminal, captures into a fixtures folder, and the WebGL
/// meshes of that terminal compared with them.
pub struct MeshReference<'a> {
    /// Folder of the captures.
    pub fixtures: PathBuf,
    /// Creates and returns the terminal setup file consumed by the reference project.
    pub setup: fn() -> PathBuf,
    pub terminal: Terminal<'a>,
    pub fonts: &'a FontSet,
    pub sprites: &'a SpriteSheet,
}

impl MeshReference<'_> {
    /// Captured meshes, recaptures first with `REFRESH_REFERENCE` set.
    #[must_use]
    pub fn captures(&self) -> Captures<MeshCase> {
        crate::refresh_reference_over("meshes", self.setup, |reference, setup| {
            capture(reference, setup, &self.fixtures)
        });

        captures(&self.fixtures)
    }

    /// Terminal meshes for each `source` line.
    #[must_use]
    pub fn build(&self, source: &str, width: f32) -> Vec<Vec<Mesh>> {
        self.terminal.build(self.fonts, self.sprites, source, width)
    }

    /// Differences between terminal meshes and `captures`.
    #[must_use]
    pub fn differences(&self, captures: &Captures<MeshCase>) -> Vec<Difference> {
        captures.differences(|differences, case, width| {
            compare(
                differences,
                self.fonts,
                &case.items,
                &self.build(&case.source, width),
            );
        })
    }

    /// Fingerprints of the assets the meshes were captured from, by asset name.
    #[must_use]
    pub fn recorded_asset_fingerprints(&self) -> BTreeMap<String, String> {
        recorded_asset_fingerprints(&self.fixtures)
    }

    /// Fingerprint of each of the terminal's assets named in `names`, by name.
    pub fn asset_fingerprints<'n>(
        &self,
        names: impl IntoIterator<Item = &'n String>,
    ) -> BTreeMap<String, String> {
        asset_fingerprints(self.fonts, names)
    }
}

/// Captures every test case through `setup` into `folder`, including meshes, asset fingerprints, and images.
///
/// # Errors
/// Returns an error if capture or file writing fails.
fn capture(reference: &Reference, setup: &Path, folder: &Path) -> Result<(), ReferenceError> {
    let captures = reference.capture(setup, &crate::cases(), WIDTH)?;

    captures.write_meshes(&folder.join(MESHES))?;
    captures.write_asset_fingerprints(&folder.join(ASSET_FINGERPRINTS))?;

    captures.copy_images(&folder.join("captures"))
}

/// Meshes captured in `folder`.
#[must_use]
fn captures(folder: &Path) -> Captures<MeshCase> {
    Captures::read(&folder.join(MESHES))
}

/// Fingerprints of the assets the meshes in `folder` were captured from, by asset name.
///
/// # Panics
/// When they aren't readable.
#[must_use]
fn recorded_asset_fingerprints(folder: &Path) -> BTreeMap<String, String> {
    read_fingerprints(&folder.join(ASSET_FINGERPRINTS))
        .expect("the asset fingerprints are readable")
}

/// Fingerprint of each asset of `fonts` (a font or the sprites) named in `names`, by name.
fn asset_fingerprints<'n>(
    fonts: &FontSet,
    names: impl IntoIterator<Item = &'n String>,
) -> BTreeMap<String, String> {
    names
        .into_iter()
        .filter_map(|name| {
            let record = match fonts.fonts.iter().find(|font| font.name == name.as_str()) {
                Some(font) => AssetRecord::of_font(font),
                None => (fonts.sprites.name == name.as_str())
                    .then(|| AssetRecord::of_sprites(&fonts.sprites))
                    .flatten(),
            }?;

            Some((name.clone(), record.fingerprint()))
        })
        .collect()
}

/// Name Unity gives the material: a font's own for its first atlas, else `TMP_MaterialManager`'s fallback material
/// "<source material> + <atlas>" (the reference project names a font's atlases "<font> Atlas", "<font> Atlas 1", …).
#[must_use]
fn material_name(fonts: &FontSet, material: MeshMaterial) -> String {
    let atlas_name = |font: usize, atlas: u8| match atlas {
        0 => format!("{} Atlas", fonts.font(font).name),
        _ => format!("{} Atlas {atlas}", fonts.font(font).name),
    };

    match material {
        MeshMaterial::Font { font, atlas: 0 } => format!("{} Material", fonts.font(font).name),
        MeshMaterial::Font { font, atlas } => format!(
            "{} Material + {}",
            fonts.font(font).name,
            atlas_name(font, atlas)
        ),
        MeshMaterial::Fallback {
            source,
            font,
            atlas,
        } => format!(
            "{} Material + {}",
            fonts.font(source).name,
            atlas_name(font, atlas)
        ),
        MeshMaterial::Sprites => format!("{} Material", fonts.sprites.name),
    }
}

/// Color TMP packs into a vertex.
#[must_use]
const fn color(packed_color: i64) -> Color {
    let [r, g, b, a] = (packed_color as u32).to_be_bytes();

    Color::new(r, g, b, a)
}

/// Records differences between TMP meshes and meshes built with `fonts`.
fn compare(
    differences: &mut Differences,
    fonts: &FontSet,
    expected: &[MeshItem],
    actual: &[Vec<Mesh>],
) {
    differences.count("items", expected.len(), actual.len());
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        let field = format!("items[{index}].meshes");

        differences.count(&field, expected.meshes.len(), actual.len());
        for (mesh, (expected_mesh, actual_mesh)) in expected.meshes.iter().zip(actual).enumerate() {
            compare_mesh(
                differences,
                fonts,
                &format!("{field}[{mesh}]"),
                expected_mesh,
                actual_mesh,
                expected.lossy_scale,
            );
        }
    }
}

fn compare_mesh(
    differences: &mut Differences,
    fonts: &FontSet,
    field: &str,
    expected: &MeshRecord,
    actual: &Mesh,
    lossy_scale: f32,
) {
    let material = material_name(fonts, actual.material);

    if expected.material != material {
        differences.push(&format!("{field}.material"), &expected.material, material);
    }

    let count = expected.scales.len();

    differences.count(&format!("{field}.vertices"), count, actual.glyphs.len());
    for (vertex, actual) in actual.glyphs.iter().enumerate().take(count) {
        for axis in 0..2 {
            let at = vertex * 2 + axis;

            differences.float(
                &format!("{field}.positions[{at}]"),
                expected.positions[at],
                actual.position[axis],
                POSITION_TOLERANCE,
            );
            differences.float(
                &format!("{field}.uvs[{at}]"),
                expected.uvs[at],
                actual.uv[axis],
                UV_TOLERANCE,
            );
        }

        let scale = expected.scales[vertex] / lossy_scale;

        differences.float(
            &format!("{field}.scales[{vertex}]"),
            scale,
            actual.scale,
            SCALE_TOLERANCE * scale.abs().max(f32::MIN_POSITIVE),
        );

        let color = color(expected.colors[vertex]);

        if color != actual.color {
            differences.push(
                &format!("{field}.colors[{vertex}]"),
                format!("{color:?}"),
                format!("{:?}", actual.color),
            );
        }
    }
}
