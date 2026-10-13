mod projection;
mod samples;
mod uniforms;

pub use projection::Projection;
pub use samples::SamplePattern;
pub use uniforms::SdfUniforms;

/// TMP 3.0.9 keyword-free Distance Field shader ported to GLSL ES 3.00.
#[derive(Debug, Clone, Copy)]
pub struct SdfShader;

impl SdfShader {
    pub const FRAGMENT: &'static str =
        include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/sdf.frag"));
    pub const VERTEX: &'static str = include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/sdf.vert"));
}

/// TMP Sprite shader ported to GLSL ES 3.00 with `SRC_ALPHA` and `ONE_MINUS_SRC_ALPHA` blending.
#[derive(Debug, Clone, Copy)]
pub struct SpriteShader;

impl SpriteShader {
    pub const FRAGMENT: &'static str =
        include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/sprite.frag"));
    pub const VERTEX: &'static str =
        include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/sprite.vert"));
}

/// Resolves multisampling by averaging sample frames onto the target with one full-target triangle.
#[derive(Debug, Clone, Copy)]
pub struct ResolveShader;

impl ResolveShader {
    pub const FRAGMENT: &'static str =
        include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/resolve.frag"));
    pub const VERTEX: &'static str =
        include_str!(concat!(env!("TEXT_MESH_SHADERS"), "/resolve.vert"));
}
