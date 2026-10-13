use text_mesh_core::Color;
use text_mesh_font::FontSet;
use text_mesh_layout::{Element, ElementGeometry, ElementKind, Layout, LayoutGeometry, Quad};

use crate::mesh::character::Character;
use crate::mesh::decorations::{DecorationManager, LineEnds};
use crate::mesh::sprite_sheet::SpriteSheet;
use crate::mesh::{Mesh, MeshMaterial, Vertex};

/// Converts laid-out text into material-grouped TMP-compatible meshes.
#[derive(Debug, Clone, Copy)]
pub struct MeshBuilder<'a> {
    fonts: &'a FontSet,
    sprites: &'a SpriteSheet,
    font_color: Color,
}

impl<'a> MeshBuilder<'a> {
    #[must_use]
    pub const fn new(fonts: &'a FontSet, sprites: &'a SpriteSheet, font_color: Color) -> Self {
        Self {
            fonts,
            sprites,
            font_color,
        }
    }

    /// Meshes of `layout`, positioned at `offset` (the item's top left
    /// in pixels).
    #[must_use]
    pub fn build(&self, layout: &Layout, geometry: &LayoutGeometry, offset: [f32; 2]) -> Vec<Mesh> {
        let characters: Vec<Character> = layout
            .elements
            .iter()
            .zip(&geometry.elements)
            .map(|(element, geometry)| Character::new(element, geometry))
            .collect();
        let lines: Vec<LineEnds> = layout
            .lines
            .iter()
            .zip(&geometry.lines)
            .map(|(line, geometry)| LineEnds {
                last: line.last,
                last_visible: geometry.last_visible,
            })
            .collect();
        let mut meshes = vec![Mesh::new(MeshMaterial::Font {
            font: self.fonts.primary,
            atlas: 0,
        })];
        let mut decorations =
            DecorationManager::new(self.fonts, self.font_color, &characters, &lines, offset);
        let mut x_scale = 0.0;

        for (index, (element, geometry)) in
            layout.elements.iter().zip(&geometry.elements).enumerate()
        {
            let material = self.material(element, geometry);
            let mesh = match meshes.iter().position(|mesh| mesh.material == material) {
                Some(mesh) => mesh,
                None => {
                    meshes.push(Mesh::new(material));

                    meshes.len() - 1
                }
            };

            if element.visible {
                if let ElementKind::Glyph { .. } = element.kind {
                    x_scale = if element.style.bold {
                        -element.scale
                    } else {
                        element.scale
                    };
                }

                let quad = self.quad(element, geometry, x_scale, offset);

                meshes[mesh].glyphs.extend(quad);
            }

            decorations.character(index, x_scale);
        }

        meshes[0].decorations = decorations.into_vertices();
        meshes.retain(|mesh| mesh.vertex_count() > 0);

        meshes
    }

    /// Primary, fallback, or sprite material TMP assigns.
    fn material(&self, element: &Element, geometry: &ElementGeometry) -> MeshMaterial {
        let ElementKind::Glyph { character, font } = element.kind else {
            return MeshMaterial::Sprites;
        };
        let primary = self.fonts.primary;
        let base = geometry.base_font.unwrap_or(primary);
        let atlas = self
            .fonts
            .font(font)
            .rect(character)
            .map_or(0, |rect| rect.atlas);

        if font == base {
            MeshMaterial::Font { font, atlas }
        } else if font == primary {
            MeshMaterial::Font { font: base, atlas }
        } else {
            MeshMaterial::Fallback {
                source: base,
                font,
                atlas,
            }
        }
    }

    fn quad(
        &self,
        element: &Element,
        geometry: &ElementGeometry,
        x_scale: f32,
        offset: [f32; 2],
    ) -> [Vertex; 4] {
        let positions = Self::positions(&geometry.quad, offset);
        let (uvs, scale, color) = match element.kind {
            ElementKind::Glyph { character, font } => (
                self.glyph_uvs(character, font, geometry),
                x_scale,
                Color {
                    a: element.style.color.a.min(self.font_color.a),
                    ..element.style.color
                },
            ),
            ElementKind::Sprite { index, color } => {
                let uvs = self
                    .fonts
                    .sprites
                    .characters
                    .get(index as usize)
                    .and_then(|character| self.sprites.uvs(character.glyph))
                    .unwrap_or_default();

                (uvs, 0.0, color)
            }
        };

        [0, 1, 2, 3].map(|corner| Vertex {
            position: positions[corner],
            uv: uvs[corner],
            scale,
            color,
        })
    }

    fn positions(quad: &Quad, offset: [f32; 2]) -> [[f32; 2]; 4] {
        [
            quad.bottom_left,
            quad.top_left,
            quad.top_right,
            quad.bottom_right,
        ]
        .map(|corner| [corner.x + offset[0], corner.y + offset[1]])
    }

    /// Computes `TMP_Text.SaveGlyphVertexInfo` UVs by expanding the glyph rect with material and style padding.
    fn glyph_uvs(&self, character: char, font: usize, geometry: &ElementGeometry) -> [[f32; 2]; 4] {
        let asset = self.fonts.font(font);
        let (Some(rect), Some(atlas)) = (asset.rect(character), asset.atlas) else {
            return [[0.0; 2]; 4];
        };
        let padding = geometry.padding;
        let style_padding = geometry.style_padding;
        let atlas_width = f32::from(atlas.width);
        let atlas_height = f32::from(atlas.height);
        let left = (f32::from(rect.x) - padding - style_padding) / atlas_width;
        let bottom = (f32::from(rect.y) - padding - style_padding) / atlas_height;
        let top =
            (f32::from(rect.y) + padding + style_padding + f32::from(rect.height)) / atlas_height;
        let right =
            (f32::from(rect.x) + padding + style_padding + f32::from(rect.width)) / atlas_width;

        [[left, bottom], [left, top], [right, top], [right, bottom]]
    }
}

#[cfg(test)]
#[path = "tests/builder.rs"]
mod tests;
