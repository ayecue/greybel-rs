use text_mesh_font::{FontSet, SdfMaterial};

use crate::mesh::MeshMaterial;

/// Distance Field material uniforms with TMP defaults and the computed scale ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfUniforms {
    pub face_color: [f32; 4],
    pub outline_color: [f32; 4],
    pub gradient_scale: f32,
    pub sharpness: f32,
    pub weight_normal: f32,
    pub weight_bold: f32,
    pub face_dilate: f32,
    pub scale_ratio_a: f32,
    pub outline_width: f32,
    pub outline_softness: f32,
    pub scale: [f32; 2],
    pub vertex_offset: [f32; 2],
}

impl SdfUniforms {
    #[must_use]
    pub fn from_material(material: &SdfMaterial) -> Self {
        Self {
            face_color: [1.0; 4],
            outline_color: [0.0, 0.0, 0.0, 1.0],
            gradient_scale: material.gradient_scale,
            sharpness: 0.0,
            weight_normal: material.weight_normal,
            weight_bold: material.weight_bold,
            face_dilate: material.face_dilate,
            scale_ratio_a: material.scale_ratio_a(),
            outline_width: material.outline_width,
            outline_softness: material.outline_softness,
            scale: [1.0; 2],
            vertex_offset: [0.0; 2],
        }
    }

    /// Builds uniforms for primary or fallback mesh materials; returns `None` for sprites.
    #[must_use]
    pub fn of(fonts: &FontSet, material: MeshMaterial) -> Option<Self> {
        let material = match material {
            MeshMaterial::Font { font, .. } => fonts.font(font).material?,
            MeshMaterial::Fallback { source, font, .. } => {
                let target = fonts.font(font).material?;

                fonts
                    .font(source)
                    .material
                    .map_or(target, |source| source.with_atlas_of(&target))
            }
            MeshMaterial::Sprites => return None,
        };

        Some(Self::from_material(&material))
    }
}
