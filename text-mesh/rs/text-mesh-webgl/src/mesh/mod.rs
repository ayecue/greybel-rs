mod builder;
mod character;
mod decorations;
mod point;
mod sprite_sheet;

#[cfg(test)]
#[path = "tests/support/shared.rs"]
mod test_support;

pub use builder::MeshBuilder;
pub use sprite_sheet::SpriteSheet;
use text_mesh_core::Color;

/// Vertex data: y-down pixel `position`, Unity-compatible `uv`, TMP `uv2.y` as `scale`, and `color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub scale: f32,
    pub color: Color,
}

impl Vertex {
    pub const COLOR_OFFSET: usize = 20;
    pub const POSITION_OFFSET: usize = 0;
    pub const SCALE_OFFSET: usize = 16;
    /// Returns [`Mesh::vertex_data`] vertex size and byte offsets for position, uv, scale, and colour.
    pub const STRIDE: usize = 24;
    pub const UV_OFFSET: usize = 8;
}

/// Material a mesh is drawn with: a font's own (`atlas` > 0 for further atlases), its fallback material, or the sprite asset's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshMaterial {
    Font {
        font: usize,
        atlas: u8,
    },
    Fallback {
        source: usize,
        font: usize,
        atlas: u8,
    },
    Sprites,
}

impl MeshMaterial {
    /// Font whose atlas the mesh samples, `None` for sprites.
    #[must_use]
    pub const fn font(&self) -> Option<(usize, u8)> {
        match *self {
            Self::Font { font, atlas } | Self::Fallback { font, atlas, .. } => Some((font, atlas)),
            Self::Sprites => None,
        }
    }
}

/// TMP mesh's glyph quads and appended decoration quads.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub material: MeshMaterial,
    pub glyphs: Vec<Vertex>,
    pub decorations: Vec<Vertex>,
}

impl Mesh {
    pub(crate) const fn new(material: MeshMaterial) -> Self {
        Self {
            material,
            glyphs: Vec::new(),
            decorations: Vec::new(),
        }
    }

    /// Vertices in drawing order.
    pub fn vertices(&self) -> impl Iterator<Item = &Vertex> {
        self.glyphs.iter().chain(&self.decorations)
    }

    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.glyphs.len() + self.decorations.len()
    }

    /// Vertices interleaved for a vertex buffer, `Vertex::STRIDE` bytes each.
    #[must_use]
    pub fn vertex_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.vertex_count() * Vertex::STRIDE);

        for vertex in self.vertices() {
            for value in [
                vertex.position[0],
                vertex.position[1],
                vertex.uv[0],
                vertex.uv[1],
                vertex.scale,
            ] {
                data.extend_from_slice(&value.to_le_bytes());
            }

            let Color { r, g, b, a } = vertex.color;

            data.extend_from_slice(&[r, g, b, a]);
        }

        data
    }

    /// Generates triangle indices for `quads` quads using (0, 1, 2) and (2, 3, 0) per quad.
    #[must_use]
    pub fn indices(quads: usize) -> Vec<u32> {
        (0..quads as u32)
            .flat_map(|quad| {
                let first = quad * 4;

                [first, first + 1, first + 2, first + 2, first + 3, first]
            })
            .collect()
    }
}
