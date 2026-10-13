use text_mesh_ansi::AnsiRenderer;
use text_mesh_core::TerminalOutput;

const SAMPLE: &str = include_str!("../../text-mesh-core/examples/sample.txt");
const COLUMNS: usize = 80;

fn main() {
    let setup = text_mesh_test_support::setup();
    let component = setup.component();
    let renderer = AnsiRenderer::new(component, setup.background).with_cells(setup.cells());
    let background_renderer = renderer.with_painted_background();
    let laid_out = background_renderer.laid_out(setup.text_layout(), COLUMNS);

    for source in SAMPLE.trim_end().lines() {
        println!("\x1b[2m{source}\x1b[0m");
        for line in TerminalOutput::new(source).lines() {
            let component_text = component.parse(&line);

            println!(
                "  {}",
                renderer.render(&component_text).replace('\n', "\n  ")
            );
            println!(
                "  {}",
                background_renderer
                    .render(&component_text)
                    .replace('\n', "\n  ")
            );
            println!(
                "  {}",
                laid_out.render(&component_text).replace('\n', "\n  ")
            );
        }

        println!();
    }
}
