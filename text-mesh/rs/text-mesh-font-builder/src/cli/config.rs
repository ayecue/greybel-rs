use std::path::{Path, PathBuf};

use serde::Deserialize;
use text_mesh_font::{FontStyleSettings, SdfMaterial};

use super::error::CliError;

/// Build described in TOML or JSON; paths are relative to the file and the command line overrides keys.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub font: Option<PathBuf>,
    pub name: Option<String>,
    pub point_size: Option<u16>,
    pub padding: Option<u16>,
    pub atlas_width: Option<u16>,
    pub atlas_height: Option<u16>,
    pub render_mode: Option<RenderModeValue>,
    pub characters: Option<String>,
    pub unicode: Option<Vec<String>>,
    pub kerning: Option<bool>,
    pub style: Option<StyleConfig>,
    pub material: Option<MaterialConfig>,
    pub rust: Option<PathBuf>,
    pub atlases: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RenderModeValue {
    Name(String),
    Unity(u32),
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StyleConfig {
    pub normal_style: Option<f32>,
    pub normal_spacing_offset: Option<f32>,
    pub bold_style: Option<f32>,
    pub bold_spacing: Option<f32>,
    pub italic_style: Option<u8>,
    pub tab_size: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialConfig {
    pub gradient_scale: Option<f32>,
    pub face_dilate: Option<f32>,
    pub outline_width: Option<f32>,
    pub outline_softness: Option<f32>,
    pub weight_normal: Option<f32>,
    pub weight_bold: Option<f32>,
}

impl Config {
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, CliError> {
        let text = std::fs::read_to_string(path).map_err(|source| CliError::Io {
            path: path.to_owned(),
            source,
        })?;
        let invalid = |message: String| CliError::Config {
            path: path.to_owned(),
            message,
        };
        let mut config: Self = match path.extension().and_then(|extension| extension.to_str()) {
            Some("toml") => toml::from_str(&text).map_err(|error| invalid(error.to_string()))?,
            Some("json") => {
                serde_json::from_str(&text).map_err(|error| invalid(error.to_string()))?
            }
            _ => {
                return Err(CliError::ConfigFormat {
                    path: path.to_owned(),
                });
            }
        };
        let base = path.parent().unwrap_or_else(|| Path::new(""));

        for relative in [&mut config.font, &mut config.rust, &mut config.atlases]
            .into_iter()
            .flatten()
        {
            *relative = base.join(&*relative);
        }

        Ok(config)
    }
}

impl StyleConfig {
    #[must_use]
    pub fn over(self, style: FontStyleSettings) -> FontStyleSettings {
        FontStyleSettings {
            normal_style: self.normal_style.unwrap_or(style.normal_style),
            normal_spacing_offset: self
                .normal_spacing_offset
                .unwrap_or(style.normal_spacing_offset),
            bold_style: self.bold_style.unwrap_or(style.bold_style),
            bold_spacing: self.bold_spacing.unwrap_or(style.bold_spacing),
            italic_style: self.italic_style.unwrap_or(style.italic_style),
            tab_size: self.tab_size.unwrap_or(style.tab_size),
        }
    }
}

impl MaterialConfig {
    #[must_use]
    pub fn over(self, material: SdfMaterial) -> SdfMaterial {
        SdfMaterial {
            gradient_scale: self.gradient_scale.unwrap_or(material.gradient_scale),
            face_dilate: self.face_dilate.unwrap_or(material.face_dilate),
            outline_width: self.outline_width.unwrap_or(material.outline_width),
            outline_softness: self.outline_softness.unwrap_or(material.outline_softness),
            weight_normal: self.weight_normal.unwrap_or(material.weight_normal),
            weight_bold: self.weight_bold.unwrap_or(material.weight_bold),
        }
    }
}
