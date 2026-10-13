use std::fmt;

use text_mesh_font::{SpriteAsset, SpriteCharacter, SpriteGlyph, SpriteTexture};

use super::RustSource;
use super::literal::Literal;

impl fmt::Display for RustSource<'_, SpriteAsset> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sprites = self.value;

        self.open(f, "::text_mesh_font::SpriteAsset")?;
        self.field(f, "name", Literal::borrowed_str(&sprites.name))?;
        self.field(f, "face", self.descend(&sprites.face))?;
        match &sprites.texture {
            Some(texture) => {
                self.field(f, "texture", format!("Some({})", self.descend(texture)))?
            }
            None => self.field(f, "texture", "None")?,
        }

        self.slice(f, "glyphs", &sprites.glyphs)?;
        self.slice(f, "characters", &sprites.characters)?;

        self.close(f)
    }
}

impl fmt::Display for RustSource<'_, SpriteGlyph> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "::text_mesh_font::SpriteGlyph {{ metrics: {}, rect: {}, scale: {} }}",
            self.descend(&self.value.metrics),
            self.descend(&self.value.rect),
            Literal::float(self.value.scale)
        )
    }
}

impl fmt::Display for RustSource<'_, SpriteCharacter> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let character = self.value;

        write!(
            f,
            "::text_mesh_font::SpriteCharacter {{ name: {}, unicode: {}, glyph: {}, scale: {} }}",
            Literal::borrowed_str(&character.name),
            character.unicode,
            character.glyph,
            Literal::float(character.scale)
        )
    }
}

impl fmt::Display for RustSource<'_, SpriteTexture> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "::text_mesh_font::SpriteTexture {{ width: {}, height: {} }}",
            self.value.width, self.value.height
        )
    }
}
