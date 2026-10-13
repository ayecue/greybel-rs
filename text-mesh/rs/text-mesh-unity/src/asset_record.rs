//! Matches captures to source assets using dimensions, glyph rectangles, and fingerprints.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use text_mesh_font::{FontAsset, SpriteAsset};

use crate::reference::{PathContext, ReferenceError};

/// Asset's atlas size and count and the rectangles of its glyphs, by unicode (a sprite's by glyph index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRecord {
    pub atlas_width: u16,
    pub atlas_height: u16,
    pub atlas_count: usize,
    pub rects: Vec<RecordedRect>,
}

/// Glyph location in its asset's atlases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct RecordedRect {
    pub unicode: u32,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub atlas: u8,
}

/// Record the reference project writes for each asset of a capture (`assets/<name>.json`).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CapturedAsset {
    name: String,
    atlas_width: u16,
    atlas_height: u16,
    atlases: Vec<String>,
    glyphs: Vec<RecordedRect>,
}

impl AssetRecord {
    /// Record of `font`, if it has atlases.
    #[must_use]
    pub fn of_font(font: &FontAsset) -> Option<Self> {
        let atlas = font.atlas?;

        Some(Self {
            atlas_width: atlas.width,
            atlas_height: atlas.height,
            atlas_count: usize::from(atlas.count),
            rects: font
                .glyphs
                .iter()
                .map(|glyph| RecordedRect {
                    unicode: u32::from(glyph.character),
                    x: glyph.rect.x,
                    y: glyph.rect.y,
                    width: glyph.rect.width,
                    height: glyph.rect.height,
                    atlas: glyph.rect.atlas,
                })
                .collect(),
        })
    }

    /// Record of `sprites`, if its texture size is known; a sprite sheet is no atlas of a font, so it counts none.
    #[must_use]
    pub fn of_sprites(sprites: &SpriteAsset) -> Option<Self> {
        let texture = sprites.texture?;

        Some(Self {
            atlas_width: texture.width,
            atlas_height: texture.height,
            atlas_count: 0,
            rects: sprites
                .glyphs
                .iter()
                .zip(0..)
                .map(|(glyph, index)| RecordedRect {
                    unicode: index,
                    x: glyph.rect.x,
                    y: glyph.rect.y,
                    width: glyph.rect.width,
                    height: glyph.rect.height,
                    atlas: 0,
                })
                .collect(),
        })
    }

    /// Computes a hex SHA-256 fingerprint from atlas dimensions, atlas count, and available glyph rectangles keyed by Unicode.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        let mut rects: Vec<&RecordedRect> =
            self.rects.iter().filter(|rect| rect.width > 0).collect();

        rects.sort();
        rects.dedup();

        let mut text = format!(
            "{} {} {}\n",
            self.atlas_width, self.atlas_height, self.atlas_count
        );

        for rect in rects {
            let _ = writeln!(
                text,
                "{} {} {} {} {} {}",
                rect.unicode, rect.x, rect.y, rect.width, rect.height, rect.atlas
            );
        }

        Sha256::digest(text.as_bytes())
            .iter()
            .fold(String::new(), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");

                hex
            })
    }
}

/// Fingerprint of every asset record (`<name>.json`) in `folder`, by asset name.
///
/// # Errors
/// When a record can't be read.
pub fn fingerprints_of_records(folder: &Path) -> Result<BTreeMap<String, String>, ReferenceError> {
    let mut fingerprints = BTreeMap::new();

    for entry in fs::read_dir(folder).at(folder)? {
        let path = entry.at(folder)?.path();

        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }

        let asset: CapturedAsset = serde_json::from_str(&fs::read_to_string(&path).at(&path)?)?;
        let record = AssetRecord {
            atlas_width: asset.atlas_width,
            atlas_height: asset.atlas_height,
            atlas_count: asset.atlases.len(),
            rects: asset.glyphs,
        };

        fingerprints.insert(asset.name, record.fingerprint());
    }

    Ok(fingerprints)
}

/// Fingerprints `Capture::write_asset_fingerprints` wrote to `path`, by asset name.
///
/// # Errors
/// When `path` can't be read.
pub fn read_fingerprints(path: &Path) -> Result<BTreeMap<String, String>, ReferenceError> {
    Ok(serde_json::from_str(&fs::read_to_string(path).at(path)?)?)
}
