//! Measures the WebGL renderings of a `compare` page against its Unity captures in headless Chrome.
//!
//!     cargo run --release -p greyhack-terminal-web --example measure -- target/compare.html
//!
//! `--chrome` names the Chrome binary, `--save` writes the renderings as PNGs to a folder, and `--gpu` renders on
//! the GPU instead of SwiftShader.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;

const CHROME: &str = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const USAGE: &str = "usage: measure <page.html> [--chrome <path>] [--save <folder>] [--gpu]";
/// The largest channel difference, out of 255, that still counts as a match; `compare.js` shades the same pixels.
const TOLERANCE: u8 = 8;

struct Options {
    page: PathBuf,
    chrome: PathBuf,
    save: Option<PathBuf>,
    gpu: bool,
}

impl Options {
    fn parse(mut arguments: impl Iterator<Item = String>) -> Self {
        let page = PathBuf::from(arguments.next().expect(USAGE));
        let mut options = Self {
            page: std::path::absolute(&page).expect("the page path is absolute"),
            chrome: PathBuf::from(CHROME),
            save: None,
            gpu: false,
        };

        while let Some(flag) = arguments.next() {
            let mut value = || arguments.next().expect(USAGE);

            match flag.as_str() {
                "--chrome" => options.chrome = PathBuf::from(value()),
                "--save" => options.save = Some(PathBuf::from(value())),
                "--gpu" => options.gpu = true,
                _ => panic!("{USAGE}"),
            }
        }

        options
    }
}

/// An image as top-down RGB pixels, alpha dropped.
struct Image {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

impl Image {
    fn decode(png: &[u8]) -> Self {
        let mut decoder = png::Decoder::new(Cursor::new(png));

        decoder.set_transformations(png::Transformations::normalize_to_color8());

        let mut reader = decoder.read_info().expect("the image is a PNG");
        let mut pixels = vec![
            0;
            reader
                .output_buffer_size()
                .expect("the image fits in memory")
        ];
        let info = reader.next_frame(&mut pixels).expect("the PNG decodes");

        pixels.truncate(info.buffer_size());

        let rgb = match info.color_type {
            png::ColorType::Rgb => pixels,
            png::ColorType::Rgba => pixels
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|&[r, g, b, _]| [r, g, b])
                .collect(),
            png::ColorType::Grayscale => pixels.iter().flat_map(|&value| [value; 3]).collect(),
            png::ColorType::GrayscaleAlpha => pixels
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|&[value, _]| [value; 3])
                .collect(),
            png::ColorType::Indexed => unreachable!("normalize_to_color8 expands palettes"),
        };

        Self {
            width: info.width,
            height: info.height,
            rgb,
        }
    }

    fn decode_base64(data: &str) -> Self {
        Self::decode(&STANDARD.decode(data).expect("the image is base64"))
    }

    fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let start = (y as usize * self.width as usize + x as usize) * 3;

        &self.rgb[start..start + 3]
    }

    fn save(&self, path: &Path) {
        let file = std::fs::File::create(path).expect("the rendering file is created");
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), self.width, self.height);

        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().expect("a PNG header is written");

        writer
            .write_image_data(&self.rgb)
            .expect("the pixels are written");
        writer.finish().expect("the PNG is finished");
    }
}

/// How a rendering differs from its capture over the area both cover, from the top left.
#[derive(Clone, Copy)]
struct Difference {
    /// The mean channel difference, out of 255.
    mean: f64,
    /// The largest channel difference.
    peak: u8,
    /// The percentage of pixels with a channel differing by more than [`TOLERANCE`].
    over: f64,
}

impl Difference {
    fn of(capture: &Image, image: &Image) -> Self {
        let width = capture.width.min(image.width);
        let height = capture.height.min(image.height);
        let mut sum = 0u64;
        let mut peak = 0;
        let mut over = 0u64;

        for y in 0..height {
            for x in 0..width {
                let mut pixel = 0;

                for (a, b) in capture.pixel(x, y).iter().zip(image.pixel(x, y)) {
                    let difference = a.abs_diff(*b);

                    sum += u64::from(difference);
                    pixel = pixel.max(difference);
                }

                peak = peak.max(pixel);
                over += u64::from(pixel > TOLERANCE);
            }
        }

        let pixels = (f64::from(width) * f64::from(height)).max(1.0);

        Self {
            #[expect(clippy::cast_precision_loss)]
            mean: sum as f64 / (pixels * 3.0),
            peak,
            #[expect(clippy::cast_precision_loss)]
            over: over as f64 / pixels * 100.0,
        }
    }

    fn mean_of(differences: &[Self]) -> Self {
        #[expect(clippy::cast_precision_loss)]
        let count = differences.len() as f64;

        Self {
            mean: differences
                .iter()
                .map(|difference| difference.mean)
                .sum::<f64>()
                / count,
            peak: differences
                .iter()
                .map(|difference| difference.peak)
                .max()
                .unwrap_or(0),
            over: differences
                .iter()
                .map(|difference| difference.over)
                .sum::<f64>()
                / count,
        }
    }

    fn print(&self, name: &str, note: &str) {
        let line = format!(
            "{name:<16} {:7.3} {:4} {:7.3}%  {note}",
            self.mean, self.peak, self.over
        );

        println!("{}", line.trim_end());
    }
}

/// A `compare` page: its Unity captures and, once Chrome has run its script, the WebGL renderings.
struct ComparePage<'o> {
    options: &'o Options,
    html: String,
}

impl<'o> ComparePage<'o> {
    fn read(options: &'o Options) -> Self {
        let html = std::fs::read_to_string(&options.page).expect("the comparison page is read");

        Self { options, html }
    }

    /// Returns each section's Unity capture as base64 PNG, keyed by section index; `None` when it was missing.
    fn captures(&self) -> Vec<(u64, Option<&str>)> {
        self.html
            .split("<section id=\"case-")
            .skip(1)
            .map(|section| {
                let (index, remainder) = section.split_once('"').expect("the section has an id");
                let section = remainder
                    .split_once("</section>")
                    .map_or(remainder, |(section, _)| section);
                let capture = section
                    .split_once("class=\"unity\" src=\"data:image/png;base64,")
                    .and_then(|(_, remainder)| remainder.split_once('"'))
                    .map(|(data, _)| data);

                (index.parse().expect("the section id is an index"), capture)
            })
            .collect()
    }

    /// Runs the page in headless Chrome and returns the renderings `compare.js` wrote to `#renderings`.
    fn renderings(&self) -> Vec<Value> {
        let profile = std::env::temp_dir().join(format!("greyhack-measure-{}", std::process::id()));
        let mut chrome = Command::new(&self.options.chrome);

        chrome
            .arg("--headless=new")
            .arg(format!("--user-data-dir={}", profile.display()));
        if !self.options.gpu {
            chrome.args(["--enable-unsafe-swiftshader", "--use-angle=swiftshader"]);
        }

        let output = chrome
            .args(["--virtual-time-budget=60000", "--dump-dom"])
            .arg(Self::file_url(&self.options.page))
            .output()
            .unwrap_or_else(|error| panic!("{}: {error}", self.options.chrome.display()));
        let _ = std::fs::remove_dir_all(&profile);

        assert!(
            output.status.success(),
            "Chrome failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );

        let dom = String::from_utf8(output.stdout).expect("the DOM is UTF-8");
        let json = dom
            .split_once("id=\"renderings\">")
            .and_then(|(_, remainder)| remainder.split_once("</script>"))
            .map(|(json, _)| json)
            .expect("the page has a #renderings script");

        match serde_json::from_str(json).expect("the renderings are JSON") {
            Value::Array(renderings) => renderings,
            Value::Object(failure) => panic!("rendering failed: {}", failure["error"]),
            _ => panic!("the page wrote no renderings"),
        }
    }

    fn file_url(path: &Path) -> String {
        let mut url = String::from("file://");

        for &byte in path.to_str().expect("the page path is UTF-8").as_bytes() {
            if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
                url.push(char::from(byte));
            } else {
                url.push_str(&format!("%{byte:02X}"));
            }
        }

        url
    }

    fn measure(&self) {
        let captures = self.captures();
        let renderings = self.renderings();
        let mut totals: Vec<(&str, Vec<Difference>)> = Vec::new();

        println!(
            "{:<16} {:>7} {:>4} {:>8}  size",
            "case", "mean", "max", ">8/255"
        );
        for entry in &renderings {
            let target = entry["target"]
                .as_str()
                .expect("the rendering has a target");
            let name = format!(
                "{} {target}",
                entry["name"].as_str().expect("the rendering has a name")
            );
            let (_, data) = entry["rendering"]
                .as_str()
                .and_then(|url| url.split_once(','))
                .expect("the rendering is a data URL");
            let image = Image::decode_base64(data);

            if let Some(folder) = &self.options.save {
                std::fs::create_dir_all(folder).expect("the save folder is created");
                image.save(&folder.join(format!("{}.png", name.replace(' ', "-"))));
            }

            let section = entry["section"]
                .as_u64()
                .expect("the rendering has a section");
            let Some(capture) = captures
                .iter()
                .find(|(index, _)| *index == section)
                .and_then(|(_, capture)| *capture)
            else {
                println!("{name:<16} no capture");
                continue;
            };
            let capture = Image::decode_base64(capture);
            let size = if (capture.width, capture.height) == (image.width, image.height) {
                String::new()
            } else {
                format!(
                    "{}×{} vs {}×{}",
                    capture.width, capture.height, image.width, image.height
                )
            };
            let difference = Difference::of(&capture, &image);

            difference.print(&name, &size);

            match totals.iter_mut().find(|(other, _)| *other == target) {
                Some((_, differences)) => differences.push(difference),
                None => totals.push((target, vec![difference])),
            }
        }

        for (target, differences) in &totals {
            Difference::mean_of(differences).print(&format!("all {target}"), "");
        }
    }
}

fn main() {
    let options = Options::parse(std::env::args().skip(1));

    ComparePage::read(&options).measure();
}
