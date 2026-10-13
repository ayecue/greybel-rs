using System.IO;
using System.Linq;
using TMPro;
using UnityEditor;
using UnityEngine;
using UnityEngine.TextCore;

namespace TextMeshReference.Editor
{
    public static class SpriteAssetBuilder
    {
        private const string AssetFolder = "Assets/Generated/Resources/Sprite Assets";
        private const string TextureFolder = "Assets/Generated/Sprites";

        public static string GetAssetPath(SpriteAssetSpec spec) =>
            $"{AssetFolder}/{spec.name}.asset";

        public static TMP_SpriteAsset Build(Setup setup)
        {
            var spec = setup.sprites;
            var assetPath = GetAssetPath(spec);
            var texturePath = $"{TextureFolder}/{spec.sheet}";
            Directory.CreateDirectory(AssetFolder);
            Directory.CreateDirectory(TextureFolder);
            File.Copy(setup.GetSpriteSheetFile(spec.sheet), texturePath, true);
            AssetDatabase.ImportAsset(texturePath);
            var importer = (TextureImporter)AssetImporter.GetAtPath(texturePath);
            importer.textureType = TextureImporterType.Default;
            importer.npotScale = TextureImporterNPOTScale.None;
            importer.textureCompression = TextureImporterCompression.Uncompressed;
            importer.alphaIsTransparency = true;
            importer.mipmapEnabled = false;
            importer.isReadable = true;
            importer.wrapMode = TextureWrapMode.Clamp;
            importer.filterMode = FilterMode.Bilinear;
            importer.SaveAndReimport();
            var texture = AssetDatabase.LoadAssetAtPath<Texture2D>(texturePath);
            if (texture.width != spec.sheetWidth || texture.height != spec.sheetHeight)
            {
                throw new InvalidDataException(
                    $"{texturePath}: {texture.width}x{texture.height}, the setup's sheet is {spec.sheetWidth}x{spec.sheetHeight}"
                );
            }

            AssetDatabase.DeleteAsset(assetPath);
            var asset = ScriptableObject.CreateInstance<TMP_SpriteAsset>();
            asset.name = spec.name;
            typeof(TMP_SpriteAsset)
                .GetField(
                    "m_Version",
                    System.Reflection.BindingFlags.Instance
                        | System.Reflection.BindingFlags.NonPublic
                )
                .SetValue(asset, "1.1.0");
            asset.spriteSheet = texture;
            foreach (var glyph in spec.glyphs)
            {
                asset.spriteGlyphTable.Add(
                    new TMP_SpriteGlyph(
                        (uint)glyph.index,
                        new GlyphMetrics(
                            glyph.width,
                            glyph.height,
                            glyph.bearingX,
                            glyph.bearingY,
                            glyph.advance
                        ),
                        new GlyphRect(glyph.x, glyph.y, glyph.rectWidth, glyph.rectHeight),
                        glyph.scale,
                        0
                    )
                );
            }
            foreach (var character in spec.characters)
            {
                var glyph = asset.spriteGlyphTable.First(candidate =>
                    candidate.index == (uint)character.glyph
                );
                asset.spriteCharacterTable.Add(
                    new TMP_SpriteCharacter(character.unicode, glyph)
                    {
                        name = character.name,
                        scale = character.scale,
                    }
                );
            }
            var material = new Material(Shader.Find("TextMeshPro/Sprite"))
            {
                name = spec.name + " Material",
            };
            material.SetTexture(ShaderUtilities.ID_MainTex, texture);
            asset.material = material;
            AssetDatabase.CreateAsset(asset, assetPath);
            AssetDatabase.AddObjectToAsset(material, asset);
            asset.UpdateLookupTables();
            EditorUtility.SetDirty(asset);
            AssetDatabase.SaveAssets();
            return asset;
        }
    }
}
