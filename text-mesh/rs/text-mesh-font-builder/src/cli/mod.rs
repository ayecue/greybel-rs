mod characters;
mod config;
mod error;
mod render_mode;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use characters::CharacterList;
use clap::Parser;
use config::{Config, RenderModeValue};
use error::CliError;
use render_mode::RenderModeName;
use text_mesh_font::RenderMode;
use text_mesh_font_builder::{BuiltFont, FontBuildSettings, FontBuilder, RustSource};

#[derive(Debug, Parser)]
#[command(
    name = "text-mesh-font-builder",
    version,
    about = "Builds a TextMesh Pro font asset (face info, glyph metrics, atlas rectangles, kerning, SDF atlases) from a TrueType or OpenType font the way Unity's Font Asset Creator does",
    after_help = "Settings come from --config (TOML or JSON with the same names in snake case: font, name, point_size, padding, atlas_width, atlas_height, render_mode, characters, unicode, kerning, [style], [material], rust, atlases; paths relative to the file) and are overridden by the flags. Without characters, the ASCII set (space to tilde) is built."
)]
pub struct Command {
    #[arg(
        long,
        value_name = "FILE",
        help = "A TOML or JSON file with the build settings"
    )]
    config: Option<PathBuf>,
    #[arg(long, value_name = "FILE", help = "The TrueType or OpenType font file")]
    font: Option<PathBuf>,
    #[arg(
        long,
        help = "The font asset's name [default: the font file's name plus \" SDF\"]"
    )]
    name: Option<String>,
    #[arg(long, help = "The sampling point size in pixels [default: 90]")]
    point_size: Option<u16>,
    #[arg(
        long,
        help = "The padding around every glyph in the atlas, in pixels [default: 9]"
    )]
    padding: Option<u16>,
    #[arg(long, help = "The atlas width in pixels [default: 1024]")]
    atlas_width: Option<u16>,
    #[arg(long, help = "The atlas height in pixels [default: 1024]")]
    atlas_height: Option<u16>,
    #[arg(
        long,
        value_parser = RenderModeName::parse,
        help = "The render mode: raster, raster-hinted, smooth, smooth-hinted, sdf, sdf8, sdf16, sdf32, sdfaa, sdfaa-hinted, or Unity's GlyphRenderMode number [default: sdfaa]"
    )]
    render_mode: Option<RenderMode>,
    #[arg(
        long,
        help = "Characters to include, in order (replaces the config's characters)"
    )]
    characters: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "A text file whose characters to include, in order"
    )]
    characters_file: Option<PathBuf>,
    #[arg(
        long,
        value_delimiter = ',',
        value_name = "RANGES",
        help = "Code points to include, in hex: 20-7E,A0-FF,2500 (added after --characters)"
    )]
    unicode: Vec<String>,
    #[arg(long, help = "Read the kern table's pair adjustments")]
    kerning: bool,
    #[arg(
        long,
        value_name = "FILE",
        help = "Write the font asset as a Rust expression (for include!) to this file"
    )]
    rust: Option<PathBuf>,
    #[arg(
        long,
        value_name = "DIR",
        help = "Write the atlases as PNGs (<name>.<index>.png, the atlas alpha in gray) into this directory"
    )]
    atlases: Option<PathBuf>,
}

impl Command {
    pub fn run(self) -> ExitCode {
        match self.build() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("text-mesh-font-builder: {error}");

                ExitCode::FAILURE
            }
        }
    }

    fn build(self) -> Result<(), CliError> {
        let config = match &self.config {
            Some(path) => Config::load(path)?,
            None => Config::default(),
        };
        let font = self
            .font
            .clone()
            .or_else(|| config.font.clone())
            .ok_or(CliError::MissingFont)?;
        let rust = self.rust.clone().or_else(|| config.rust.clone());
        let atlases = self.atlases.clone().or_else(|| config.atlases.clone());

        if rust.is_none() && atlases.is_none() {
            return Err(CliError::NoOutput);
        }

        let settings = self.settings(&config, &font)?;
        let data = Self::read(&font)?;
        let built_font = FontBuilder::new(&data, settings)?.build()?;

        if let Some(path) = &rust {
            Self::write_rust(path, &font, &built_font)?;
        }

        if let Some(directory) = &atlases {
            Self::write_atlases(directory, &built_font)?;
        }

        Self::summarize(&built_font);

        Ok(())
    }

    fn settings(&self, config: &Config, font: &Path) -> Result<FontBuildSettings, CliError> {
        let render_mode = match (self.render_mode, &config.render_mode) {
            (Some(mode), _) => mode,
            (None, Some(RenderModeValue::Name(name))) => RenderModeName::parse(name)?,
            (None, Some(RenderModeValue::Unity(value))) => RenderMode::from_unity(*value)
                .ok_or_else(|| CliError::RenderMode(value.to_string()))?,
            (None, None) => FontBuildSettings::DEFAULT_RENDER_MODE,
        };
        let name = self
            .name
            .clone()
            .or_else(|| config.name.clone())
            .unwrap_or_else(|| {
                let stem = font.file_stem().map_or_else(
                    || "Font".to_owned(),
                    |stem| stem.to_string_lossy().into_owned(),
                );
                let suffix = if render_mode.is_distance_field() {
                    " SDF"
                } else {
                    " Bitmap"
                };

                format!("{stem}{suffix}")
            });
        let characters = self.characters(config)?;
        let mut settings = FontBuildSettings::new(name, characters)
            .with_sampling_size(
                self.point_size
                    .or(config.point_size)
                    .unwrap_or(FontBuildSettings::DEFAULT_POINT_SIZE),
                self.padding
                    .or(config.padding)
                    .unwrap_or(FontBuildSettings::DEFAULT_PADDING),
            )
            .with_atlas(
                self.atlas_width
                    .or(config.atlas_width)
                    .unwrap_or(FontBuildSettings::DEFAULT_ATLAS_SIZE),
                self.atlas_height
                    .or(config.atlas_height)
                    .unwrap_or(FontBuildSettings::DEFAULT_ATLAS_SIZE),
                render_mode,
            )
            .with_kerning(self.kerning || config.kerning.unwrap_or(false));

        if let Some(style) = config.style {
            settings = settings.with_style(style.over(FontBuildSettings::DEFAULT_STYLE));
        }

        if let Some(material) = config.material {
            let material = material.over(settings.default_material());

            settings = settings.with_material(material);
        }

        Ok(settings)
    }

    fn characters(&self, config: &Config) -> Result<Vec<char>, CliError> {
        let mut list = CharacterList::default();
        let from_flags =
            self.characters.is_some() || self.characters_file.is_some() || !self.unicode.is_empty();

        if from_flags {
            if let Some(text) = &self.characters {
                list.push_text(text);
            }

            if let Some(path) = &self.characters_file {
                let text = std::fs::read_to_string(path).map_err(|source| CliError::Io {
                    path: path.clone(),
                    source,
                })?;

                list.push_text(&text);
            }

            for range in &self.unicode {
                list.push_range(range)?;
            }
        } else {
            if let Some(text) = &config.characters {
                list.push_text(text);
            }

            for range in config.unicode.iter().flatten() {
                list.push_range(range)?;
            }
        }

        Ok(if list.is_empty() {
            FontBuildSettings::ascii()
        } else {
            list.into_characters()
        })
    }

    fn read(path: &Path) -> Result<Vec<u8>, CliError> {
        std::fs::read(path).map_err(|source| CliError::Io {
            path: path.to_owned(),
            source,
        })
    }

    fn write(path: &Path, bytes: &[u8]) -> Result<(), CliError> {
        let io = |source| CliError::Io {
            path: path.to_owned(),
            source,
        };

        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(io)?;
        }

        std::fs::write(path, bytes).map_err(io)
    }

    fn write_rust(path: &Path, font: &Path, built_font: &BuiltFont) -> Result<(), CliError> {
        let file = font
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let settings = built_font.asset.atlas.map_or_else(String::new, |atlas| {
            format!(
                " ({} pt, padding {}, {}x{} {})",
                built_font.asset.face.point_size,
                atlas.padding,
                atlas.width,
                atlas.height,
                RenderModeName::of(atlas.render_mode)
            )
        });
        let source = format!(
            "// Generated by text-mesh-font-builder from {file}{settings}.\n{}\n",
            RustSource::new(&built_font.asset)
        );

        Self::write(path, source.as_bytes())
    }

    fn write_atlases(directory: &Path, built_font: &BuiltFont) -> Result<(), CliError> {
        for (index, atlas) in built_font.atlases.iter().enumerate() {
            let path = directory.join(format!("{}.{index}.png", built_font.asset.name));

            Self::write(&path, &atlas.encode_png()?)?;
        }

        Ok(())
    }

    fn summarize(built_font: &BuiltFont) {
        eprintln!(
            "{}: {} glyphs, {} kerning pairs, {} atlas{}",
            built_font.asset.name,
            built_font.asset.glyphs.len(),
            built_font.asset.kerning.len(),
            built_font.atlases.len(),
            if built_font.atlases.len() == 1 {
                ""
            } else {
                "es"
            }
        );
        if !built_font.unmapped_characters.is_empty() {
            let unmapped_text: String = built_font.unmapped_characters.iter().collect();

            eprintln!("missing from the font: {unmapped_text:?}");
        }
    }
}
