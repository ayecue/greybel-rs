use std::collections::BTreeMap;

use crate::sgr::Sgr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) character: char,
    pub(crate) sgr: Sgr,
}

#[derive(Debug, Default)]
pub(crate) struct Grid {
    rows: BTreeMap<i32, Vec<Option<Cell>>>,
}

impl Grid {
    pub(crate) fn put(&mut self, row: i32, column: usize, cell: Cell, overwrite: bool) {
        let cells = self.rows.entry(row).or_default();

        if cells.len() <= column {
            cells.resize(column + 1, None);
        }

        let slot = &mut cells[column];

        if overwrite || slot.is_none() {
            *slot = Some(cell);
        }
    }

    pub(crate) fn ensure_row(&mut self, row: i32) {
        self.rows.entry(row).or_default();
    }

    pub(crate) fn write(&self, blank: Sgr, out: &mut String) {
        let (Some(&first), Some(&last)) = (self.rows.keys().next(), self.rows.keys().next_back())
        else {
            return;
        };
        let mut active = Sgr::default();

        for row in first..=last {
            if row != first {
                if !active.is_plain() {
                    Sgr::reset(out);
                    active = Sgr::default();
                }

                out.push('\n');
            }

            for cell in self.rows.get(&row).into_iter().flatten() {
                let cell = cell.unwrap_or(Cell {
                    character: ' ',
                    sgr: blank,
                });

                if cell.sgr != active {
                    if !active.is_plain() {
                        Sgr::reset(out);
                    }

                    if !cell.sgr.is_plain() {
                        cell.sgr.write(out);
                    }

                    active = cell.sgr;
                }

                out.push(cell.character);
            }
        }

        if !active.is_plain() {
            Sgr::reset(out);
        }
    }
}

pub(crate) struct QuarterTurn;

impl QuarterTurn {
    const HORIZONTAL: [(char, char); 6] = [
        ('—', '│'),
        ('–', '│'),
        ('-', '│'),
        ('─', '│'),
        ('━', '┃'),
        ('_', '│'),
    ];
    const VERTICAL: [(char, char); 3] = [('|', '─'), ('│', '─'), ('┃', '━')];

    pub(crate) fn apply(character: char, rotation: f32) -> char {
        let angle = rotation.rem_euclid(180.0);

        if (angle - 90.0).abs() > 10.0 {
            return character;
        }

        Self::HORIZONTAL
            .iter()
            .chain(&Self::VERTICAL)
            .find(|(from, _)| *from == character)
            .map_or(character, |&(_, to)| to)
    }
}
