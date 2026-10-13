//! The same font file and settings always build the same Rust source and byte-identical atlas PNGs.

use crate::support::determinism::Output;
use crate::support::recordings;

#[test]
fn should_build_identical_output_twice() {
    let fonts: Vec<_> = recordings()
        .into_iter()
        .filter(|recording| {
            ["FiraMono-Regular SDF", "Sarabun-Regular SDF"].contains(&recording.name.as_str())
        })
        .collect();

    let outputs: Vec<(Output, Output)> = fonts
        .iter()
        .map(|recording| {
            (
                Output::of(&recording.build()),
                Output::of(&recording.build()),
            )
        })
        .collect();

    assert_eq!(outputs.len(), 2);
    for (first, second) in outputs {
        assert!(!first.pngs.is_empty());
        assert_eq!(first.source, second.source);
        assert_eq!(first.pngs, second.pngs);
    }
}
