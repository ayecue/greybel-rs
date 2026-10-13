use text_mesh_core::{Color, Run};

use crate::support::parse;

pub fn sprites(text: &str) -> Vec<(u32, Color)> {
    parse(text)
        .runs
        .into_iter()
        .filter_map(|run| match run {
            Run::Sprite { index, color, .. } => Some((index, color)),
            _ => None,
        })
        .collect()
}
