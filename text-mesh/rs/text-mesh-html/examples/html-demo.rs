use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use text_mesh_core::Color;
use text_mesh_html::HtmlRenderer;

const SAMPLE: &str = include_str!("../../text-mesh-core/examples/sample.txt");

fn css(Color { r, g, b, .. }: Color) -> String {
    format!("rgb({r},{g},{b})")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn main() {
    let setup = text_mesh_test_support::setup();
    let renderer = HtmlRenderer::new(setup.component());
    let font = &setup.fonts.fonts[setup.fonts.primary];
    let cells = setup.cells();
    let mut page = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>TextMesh Pro demo</title><style>",
    );

    for (asset, source) in &setup.sources {
        let data = STANDARD.encode(text_mesh_test_support::font_file(source));
        let _ = write!(
            page,
            "@font-face{{font-family:'{asset}';src:url(data:font/ttf;base64,{data})}}"
        );
    }

    let _ = write!(
        page,
        "body{{margin:0;padding:16px;font:14px system-ui;background:#1e1e1e;color:#ddd}}\
         section{{margin-bottom:14px}}\
         pre{{margin:0 0 4px;font:12px monospace;white-space:pre-wrap;word-break:break-all;color:#999}}\
         .terminal{{padding:6px 10px;overflow-x:auto;background:{};color:{};font-family:'{}';font-size:{}px;line-height:{};white-space:pre-wrap;overflow-wrap:anywhere}}\
         .sprite{{display:inline-block;width:0;overflow:visible}}.sprite::before{{content:'';display:inline-block;width:.6em;height:1.15em;vertical-align:text-bottom;background:currentColor}}\
         </style>",
        css(setup.background),
        css(setup.text),
        font.name,
        setup.settings.font_size,
        cells.line_height,
    );

    for source in SAMPLE.trim_end().lines() {
        let _ = write!(
            page,
            "<section><pre>{}</pre><div class=\"terminal\">{}</div></section>",
            escape(source),
            renderer.render_terminal(source)
        );
    }

    page.push('\n');

    match std::env::args().nth(1) {
        Some(path) => std::fs::write(&path, page).expect("the demo page is written"),
        None => print!("{page}"),
    }
}
