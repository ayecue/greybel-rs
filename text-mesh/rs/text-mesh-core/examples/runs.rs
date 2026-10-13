use text_mesh_core::TerminalOutput;

const SAMPLE: &str = include_str!("sample.txt");

fn main() {
    let component = text_mesh_test_support::setup().component();

    for line in TerminalOutput::new(SAMPLE.trim_end()).lines() {
        println!("{line}");
        for run in component.parse(&line).runs {
            println!("    {run:?}");
        }
    }
}
