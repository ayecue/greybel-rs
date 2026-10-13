//! Writes an HTML comparison of Unity captures with this crate's WebGL, canvas, and markup output. WebGL uses the
//! TypeScript package's `WebGlRenderer`; build its classic script first with `pnpm build` in
//! `greybel/ts/greyhack-terminal-web`. The `measure` example measures the page's WebGL renderings against its Unity
//! captures.
//!
//!     cargo run --release -p greyhack-terminal-web --example compare -- target/compare.html --unity

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use greyhack_text_mesh_assets::{
    AtlasFile, FontFile, TERMINAL_TEXT, TERMINAL_THEME, TEXT_ASSETS, TerminalText,
};
use serde_json::{Value, json};
use text_mesh_canvas::CanvasPainter;
use text_mesh_core::{Color, TerminalOutput, TextComponent};
use text_mesh_font::{FontAsset, FontSet, Glyph, RenderMode};
use text_mesh_font_builder::{AtlasImage, BuiltFont, FontBuildSettings, FontBuilder};
use text_mesh_html::HtmlRenderer;
use text_mesh_layout::{LayoutGeometry, TerminalItem};
use text_mesh_webgl::{MeshBuilder, Projection, SamplePattern, SdfUniforms, SpriteSheet};

#[path = "../tests/support/meshes.rs"]
mod support;

use support::{fonts, game_setup, lay_out, reference, sprite_sheet};

const SAMPLE: &str = include_str!("sample.txt");
const SCRIPT: &str = include_str!("compare.js");
/// Loads the TypeScript package's built classic script containing `greyhackTerminalWeb.WebGlRenderer`.
static RENDERER_SCRIPT: LazyLock<String> = LazyLock::new(|| {
    let script = Path::new(env!("GREYBEL_TS")).join("greyhack-terminal-web/dist/index.iife.js");

    std::fs::read_to_string(&script).unwrap_or_else(|error| {
        panic!(
            "{}: {error}; run `pnpm build` in greybel/ts/greyhack-terminal-web",
            script.display()
        )
    })
});
const CANVAS_SCRIPT: &str = include_str!("compare-canvas.js");
const USAGE: &str = "usage: compare <output.html> [--unity | --samples <folder>] [--width <pixels>] \
     [--pixel-ratio <ratio>] [--msaa-samples <samples>] [--anisotropy <taps>]";

struct Options {
    output: PathBuf,
    samples: Option<PathBuf>,
    unity: bool,
    width: f32,
    pixel_ratio: f32,
    sample_count: u8,
    anisotropy: u8,
}

impl Options {
    fn parse(mut arguments: impl Iterator<Item = String>) -> Self {
        let mut options = Self {
            output: PathBuf::from(arguments.next().expect(USAGE)),
            samples: None,
            unity: false,
            width: 1000.0,
            pixel_ratio: 1.0,
            sample_count: 2,
            anisotropy: 16,
        };

        while let Some(flag) = arguments.next() {
            let mut value = || arguments.next().expect(USAGE);

            match flag.as_str() {
                "--samples" => options.samples = Some(PathBuf::from(value())),
                "--unity" => options.unity = true,
                "--width" => options.width = value().parse().expect(USAGE),
                "--pixel-ratio" => options.pixel_ratio = value().parse().expect(USAGE),
                "--msaa-samples" => options.sample_count = value().parse().expect(USAGE),
                "--anisotropy" => options.anisotropy = value().parse().expect(USAGE),
                _ => panic!("{USAGE}"),
            }
        }

        options
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtlasSource {
    Game,
    FontBuild,
}

impl AtlasSource {
    const ALL: [Self; 2] = [Self::Game, Self::FontBuild];

    const fn target(self) -> &'static str {
        match self {
            Self::Game => "webgl",
            Self::FontBuild => "built",
        }
    }
}

/// Game fonts with atlases rebuilt by `text-mesh-font-builder` from game settings; metrics and materials remain
/// unchanged.
struct FontBuildSet {
    fonts: FontSet,
    atlases: Vec<Vec<AtlasImage>>,
}

impl FontBuildSet {
    fn new() -> Self {
        let specs: Value =
            serde_json::from_str(TerminalText::game_setup()).expect("the game setup is JSON");
        let mut fonts = TERMINAL_TEXT.font_set.clone();
        let mut atlases = Vec::new();
        let assets: Vec<FontAsset> = fonts
            .fonts
            .iter()
            .map(|font| match Self::build(&specs, font) {
                Some(built_font) => {
                    atlases.push(built_font.atlases);

                    Self::with_atlas_of(font, &built_font.asset)
                }
                None => {
                    eprintln!("{}: not built, drawn without glyphs", font.name);
                    atlases.push(Vec::new());

                    font.clone()
                }
            })
            .collect();

        fonts.fonts = Cow::Owned(assets);

        Self { fonts, atlases }
    }

    fn build(specs: &Value, font: &FontAsset) -> Option<BuiltFont> {
        let spec = ["fonts", "bundled"]
            .iter()
            .filter_map(|list| specs[list].as_array())
            .flatten()
            .find(|spec| spec["name"] == font.name.as_ref())?;
        let file = FontFile::find(&font.name)?;
        let number = |key: &str| {
            spec[key]
                .as_u64()
                .and_then(|value| u16::try_from(value).ok())
        };
        let characters = spec["characters"]
            .as_array()?
            .iter()
            .filter_map(|character| character["unicode"].as_u64())
            .filter_map(|unicode| char::from_u32(u32::try_from(unicode).ok()?));
        let mut settings = FontBuildSettings::new(font.name.to_string(), characters);

        settings.point_size = number("pointSize")?;
        settings.padding = number("padding")?;
        settings.atlas_width = number("atlasWidth")?;
        settings.atlas_height = number("atlasHeight")?;
        settings.render_mode =
            RenderMode::from_unity(u32::try_from(spec["renderMode"].as_u64()?).ok()?)?;

        FontBuilder::new(file.data, settings).ok()?.build().ok()
    }

    fn with_atlas_of(font: &FontAsset, atlas_source: &FontAsset) -> FontAsset {
        let glyphs: Vec<Glyph> = font
            .glyphs
            .iter()
            .map(|glyph| Glyph {
                rect: atlas_source
                    .glyphs
                    .iter()
                    .find(|other| other.character == glyph.character)
                    .map_or_else(Default::default, |other| other.rect),
                ..*glyph
            })
            .collect();
        let mut font = font.clone().with_glyphs(glyphs);

        font.atlas = atlas_source.atlas;

        font
    }
}

struct ComparePage {
    fonts: FontSet,
    built_fonts: FontBuildSet,
    sprites: SpriteSheet,
    component: TextComponent<'static>,
    painter: CanvasPainter<'static>,
    html: HtmlRenderer<'static>,
    pixel_ratio: f32,
    sample_count: u8,
    anisotropy: u8,
    sections: String,
    case_count: usize,
    cases: Vec<Value>,
    /// Atlas images referenced by sampled meshes, keyed by atlas source, font, and index.
    atlases: BTreeMap<(&'static str, usize, u8), Value>,
}

impl ComparePage {
    fn new(options: &Options) -> Self {
        let component = TERMINAL_THEME.component(&TEXT_ASSETS);

        Self {
            fonts: fonts().clone(),
            built_fonts: FontBuildSet::new(),
            sprites: sprite_sheet(),
            component,
            painter: CanvasPainter::new(&TERMINAL_TEXT.font_set),
            html: HtmlRenderer::new(component),
            pixel_ratio: options.pixel_ratio,
            sample_count: options.sample_count,
            anisotropy: options.anisotropy,
            sections: String::new(),
            case_count: 0,
            cases: Vec::new(),
            atlases: BTreeMap::new(),
        }
    }

    fn add(&mut self, name: &str, source: &str, capture: &Path, width: f32) {
        let items = lay_out(&self.fonts, source, width);
        let padding = TERMINAL_TEXT.frame.padding;
        let height = items
            .last()
            .map_or(2.0 * padding, |(item, _)| {
                item.top + item.layout.height + padding
            })
            .ceil();
        let index = self.case_count;

        self.case_count += 1;
        for atlases in AtlasSource::ALL {
            let meshes = self.meshes(atlases, &items, padding);

            self.cases.push(json!({
                "name": name,
                "section": index,
                "target": atlases.target(),
                "frame": self.frame(width, height),
                "meshes": meshes,
            }));
        }

        let unity = match std::fs::read(capture) {
            Ok(png) => format!(
                "<img class=\"unity\" src=\"data:image/png;base64,{}\">",
                STANDARD.encode(png)
            ),
            Err(_) => format!(
                "<pre>missing {}</pre>",
                Self::escape(&capture.display().to_string())
            ),
        };
        let commands: Vec<_> = items
            .iter()
            .flat_map(|(item, _)| self.painter.paint(&item.layout, padding, item.top))
            .collect();
        let markup: String = TerminalOutput::new(source)
            .lines()
            .iter()
            .map(|line| {
                format!(
                    "<div class=\"line\"><div>{}</div></div>",
                    self.html.render(&self.component.parse(line))
                )
            })
            .collect();
        let _ = write!(
            self.sections,
            "<section id=\"case-{index}\"><h2>{}</h2><pre>{}</pre>\
             <div class=\"label\">Unity (TextMesh Pro)</div>{unity}\
             <div class=\"label\">text-mesh-webgl, over the game's atlases</div><img class=\"webgl\">\
             <div class=\"label\">difference to Unity (grey: 1–8/255, red: more) <span class=\"stats-webgl\"></span></div><canvas class=\"diff-webgl\"></canvas>\
             <div class=\"label\">text-mesh-webgl, over the atlases text-mesh-font-builder built</div><img class=\"built\">\
             <div class=\"label\">difference to Unity (grey: 1–8/255, red: more) <span class=\"stats-built\"></span></div><canvas class=\"diff-built\"></canvas>\
             <div class=\"label\">text-mesh-canvas</div>\
             <canvas data-commands=\"commands-{index}\" data-width=\"{width}\" data-height=\"{height}\" data-background=\"{}\"></canvas>\
             <script type=\"application/json\" id=\"commands-{index}\">{}</script>\
             <div class=\"label\">text-mesh-html</div><div class=\"terminal\" style=\"width:{width}px\">{markup}</div></section>",
            Self::escape(name),
            Self::escape(source),
            Self::css(TERMINAL_THEME.background),
            serde_json::to_string(&commands)
                .expect("drawing commands serialize")
                .replace("</", "<\\/"),
        );
    }

    fn font_set(&self, atlases: AtlasSource) -> &FontSet {
        match atlases {
            AtlasSource::Game => &self.fonts,
            AtlasSource::FontBuild => &self.built_fonts.fonts,
        }
    }

    fn meshes(
        &mut self,
        atlases: AtlasSource,
        items: &[(TerminalItem, LayoutGeometry)],
        padding: f32,
    ) -> Vec<Value> {
        let raw_meshes: Vec<_> = {
            let builder =
                MeshBuilder::new(self.font_set(atlases), &self.sprites, TERMINAL_THEME.text);

            items
                .iter()
                .flat_map(|(item, geometry)| {
                    builder.build(&item.layout, geometry, [padding, item.top])
                })
                .collect()
        };
        let mut meshes = Vec::new();

        for mesh in raw_meshes
            .into_iter()
            .filter(|mesh| mesh.vertex_count() > 0)
        {
            let font = mesh.material.font().map(|(font, atlas)| {
                self.add_atlas(atlases, font, atlas);

                let uniforms = SdfUniforms::of(self.font_set(atlases), mesh.material);

                json!({
                    "font": font,
                    "atlas": atlas,
                    "uniforms": uniforms.map(|uniforms| json!({
                        "faceColor": uniforms.face_color,
                        "outlineColor": uniforms.outline_color,
                        "gradientScale": uniforms.gradient_scale,
                        "sharpness": uniforms.sharpness,
                        "weightNormal": uniforms.weight_normal,
                        "weightBold": uniforms.weight_bold,
                        "faceDilate": uniforms.face_dilate,
                        "scaleRatioA": uniforms.scale_ratio_a,
                        "outlineWidth": uniforms.outline_width,
                        "outlineSoftness": uniforms.outline_softness,
                        "scale": uniforms.scale,
                        "vertexOffset": uniforms.vertex_offset,
                    })),
                })
            });

            meshes.push(json!({
                "font": font,
                "vertices": STANDARD.encode(mesh.vertex_data()),
            }));
        }

        meshes
    }

    /// Builds the terminal protocol frame for a `width` × `height` CSS-pixel case.
    fn frame(&self, width: f32, height: f32) -> Value {
        let projection = Projection {
            width,
            height,
            pixel_ratio: self.pixel_ratio,
        };
        let [screen_width, screen_height] = projection.screen();
        let background = TERMINAL_THEME.background;

        json!({
            "width": screen_width,
            "height": screen_height,
            "projection": projection.matrix(),
            "samples": SamplePattern::offsets(self.sample_count)
                .iter()
                .map(|&[x, y]| [x * self.pixel_ratio, -y * self.pixel_ratio])
                .collect::<Vec<_>>(),
            "anisotropy": self.anisotropy,
            "background": [background.r, background.g, background.b, background.a],
        })
    }

    fn css(Color { r, g, b, .. }: Color) -> String {
        format!("rgb({r},{g},{b})")
    }

    fn font_faces() -> String {
        let mut faces = String::new();

        for font in FontFile::all() {
            let data = STANDARD.encode(font.data);

            for asset in font.assets {
                let _ = write!(
                    faces,
                    "@font-face{{font-family:'{asset}';src:url(data:font/ttf;base64,{data})}}"
                );
            }
        }

        faces
    }

    fn terminal_style() -> String {
        let theme = *TERMINAL_THEME;
        let fallbacks = TERMINAL_TEXT
            .font_set
            .fonts
            .iter()
            .filter(|font| !font.name.starts_with("LiberationSans"))
            .map(|font| format!("'{}'", font.name))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            ".terminal{{box-sizing:border-box;padding:10px;background:{};color:{};font-family:{fallbacks};font-size:{}px;line-height:{};white-space:pre-wrap;overflow-wrap:anywhere;overflow:hidden}}\
             .line{{min-height:24px;display:flex;align-items:center}}.line>div{{flex:1;min-width:0}}\
             .sprite{{display:inline-block;width:0;overflow:visible}}\
             .sprite::before{{content:'';display:inline-block;width:.6em;height:1.15em;vertical-align:text-bottom;background:currentColor}}",
            Self::css(theme.background),
            Self::css(theme.text),
            theme.font_size,
            theme.cells.line_height,
        )
    }

    /// Adds the selected atlas's top-down alpha pixels to the page, keyed by source, font, and index.
    fn add_atlas(&mut self, atlases: AtlasSource, font: usize, atlas: u8) {
        let key = (atlases.target(), font, atlas);

        if self.atlases.contains_key(&key) {
            return;
        }

        let (width, height, alpha) = match atlases {
            AtlasSource::Game => {
                let file =
                    AtlasFile::find(&self.fonts.font(font).name, atlas).expect("the game's atlas");

                (file.width, file.height, file.alpha)
            }
            AtlasSource::FontBuild => {
                let image = &self.built_fonts.atlases[font][usize::from(atlas)];

                (image.width, image.height, image.pixels.as_slice())
            }
        };
        let value = json!({
            "font": font,
            "index": atlas,
            "width": width,
            "height": height,
            "alpha": STANDARD.encode(alpha),
        });

        self.atlases.insert(key, value);
    }

    /// Returns the game sprite sheet as top-down RGBA pixels for the comparison page.
    fn sprite_sheet() -> Value {
        let sheet = TERMINAL_TEXT.sprite_sheet;

        json!({ "width": sheet.width, "height": sheet.height, "pixels": STANDARD.encode(sheet.rgba) })
    }

    fn escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    fn finish(self) -> String {
        let mut atlases = serde_json::Map::new();

        for ((target, _, _), atlas) in self.atlases {
            if let Value::Array(list) = atlases
                .entry(target)
                .or_insert_with(|| Value::Array(Vec::new()))
            {
                list.push(atlas);
            }
        }

        let data = json!({
            "spriteSheet": Self::sprite_sheet(),
            "atlases": atlases,
            "cases": self.cases,
        });
        let mut page = format!(
            "<!doctype html><meta charset=\"utf-8\"><title>TextMesh Pro: Unity, text-mesh-webgl, text-mesh-canvas and text-mesh-html</title><style>{}\
             body{{margin:0;padding:16px;font:14px system-ui;background:#1e1e1e;color:#ddd}}\
             section{{margin-bottom:24px}}h2{{font-size:13px;margin:0}}\
             pre{{margin:0 0 4px;font:12px monospace;white-space:pre-wrap;word-break:break-all;color:#999}}\
             .label{{font:11px system-ui;color:#888;margin:6px 0 2px}}\
             img,canvas{{display:block;image-rendering:pixelated}}[hidden]{{display:none}}{}</style>",
            Self::font_faces(),
            Self::terminal_style(),
        );

        page.push_str(&self.sections);

        let _ = write!(
            page,
            "<script type=\"application/json\" id=\"meshes\">{}</script>\
             <script type=\"application/json\" id=\"renderings\"></script><script>{}</script><script>{SCRIPT}</script><script>{CANVAS_SCRIPT}</script>",
            data.to_string().replace("</", "<\\/"),
            RENDERER_SCRIPT.replace("</script", "<\\/script"),
        );

        page
    }
}

fn main() {
    let options = Options::parse(std::env::args().skip(1));
    let reference = reference();
    let captures = reference.captures();
    let mut page = ComparePage::new(&options);
    let capture_folder = options.unity.then(|| {
        let captures = text_mesh_unity::Reference::locate()
            .and_then(|reference| {
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let width = options.width as u32;

                reference.capture(&game_setup(), SAMPLE, width)
            })
            .expect("the reference project captures the sample");
        let folder = std::env::temp_dir().join(format!("greyhack-samples-{}", std::process::id()));

        captures
            .copy_images(&folder)
            .expect("the captures are copied");

        folder
    });

    if let Some(samples) = options.samples.as_ref().or(capture_folder.as_ref()) {
        for (line, source) in SAMPLE.trim_end().lines().enumerate() {
            let capture = samples.join(format!("line-{line:03}.png"));

            page.add(&format!("sample {line}"), source, &capture, options.width);
        }
    }

    for (index, case) in captures.cases.iter().enumerate() {
        let capture = reference.fixtures.join("captures").join(&case.image);

        page.add(
            &format!("case {index}"),
            &case.source,
            &capture,
            captures.width,
        );
    }

    std::fs::write(&options.output, page.finish()).expect("the comparison page is written");
}
