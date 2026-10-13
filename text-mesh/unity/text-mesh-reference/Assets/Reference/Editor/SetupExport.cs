using System.Collections.Generic;
using System.IO;
using System.Linq;
using TMPro;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering;

namespace TextMeshReference.Editor
{
    public static class SetupExport
    {
        public static void Run()
        {
            var setup = Setup.FromCommandLine();
            ProjectSetup.Generate(setup);
            var output = CommandLine.GetRequiredArgument("-exportOutput");
            var atlases = Path.Combine(output, "atlases");
            if (Directory.Exists(atlases))
            {
                Directory.Delete(atlases, true);
            }
            Directory.CreateDirectory(atlases);
            var exportedSetup = new Setup
            {
                text = setup.text,
                frame = setup.frame,
                theme = setup.theme,
                sprites = setup.sprites,
                terminal = setup.terminal,
                globalFallbacks = setup.globalFallbacks,
                missingGlyphCharacter = setup.missingGlyphCharacter,
                fonts = setup
                    .fonts.Select(spec => Export(spec, LoadGeneratedAsset(spec.name), atlases))
                    .ToArray(),
                bundled = setup
                    .bundled.Select(spec =>
                        Export(
                            spec,
                            Resources.Load<TMP_FontAsset>("Fonts & Materials/" + spec.name),
                            atlases
                        )
                    )
                    .ToArray(),
                textAssets = ExportTextAssets(setup),
            };
            File.WriteAllText(
                Path.Combine(output, "text-mesh.json"),
                JsonUtility.ToJson(exportedSetup, true) + "\n"
            );
            Debug.Log($"Exported the setup into {output}");
        }

        private static TMP_FontAsset LoadGeneratedAsset(string name)
        {
            var asset = AssetDatabase.LoadAssetAtPath<TMP_FontAsset>(
                $"{FontAssetBuilder.AssetFolder}/{name}.asset"
            );
            if (asset == null)
            {
                throw new System.InvalidOperationException($"{name} was not generated");
            }
            return asset;
        }

        private static FontSpec Export(FontSpec spec, TMP_FontAsset asset, string atlases)
        {
            var face = asset.faceInfo;
            var unicodeOf = new Dictionary<uint, uint>();
            var characters = new List<CharacterSpec>();
            foreach (var character in asset.characterTable.OrderBy(character => character.unicode))
            {
                var glyph = character.glyph;
                unicodeOf[glyph.index] = character.unicode;
                characters.Add(
                    new CharacterSpec
                    {
                        unicode = character.unicode,
                        advance = glyph.metrics.horizontalAdvance,
                        bearingX = glyph.metrics.horizontalBearingX,
                        bearingY = glyph.metrics.horizontalBearingY,
                        width = glyph.metrics.width,
                        height = glyph.metrics.height,
                        x = glyph.glyphRect.x,
                        y = glyph.glyphRect.y,
                        rectWidth = glyph.glyphRect.width,
                        rectHeight = glyph.glyphRect.height,
                        atlas = glyph.atlasIndex,
                    }
                );
            }
            var files = new List<string>();
            for (
                var index = 0;
                index < asset.atlasTextures.Length && asset.atlasTextures[index] != null;
                index++
            )
            {
                var file = $"{spec.name}.{index}.png";
                File.WriteAllBytes(
                    Path.Combine(atlases, file),
                    AtlasPng.ReadAlphaPng(asset.atlasTextures[index])
                );
                files.Add(file);
            }
            var material = asset.material;
            return new FontSpec
            {
                name = spec.name,
                source = spec.source,
                family = face.familyName,
                style = face.styleName,
                pointSize = face.pointSize,
                padding = asset.atlasPadding,
                atlasWidth = asset.atlasWidth,
                atlasHeight = asset.atlasHeight,
                renderMode = (int)asset.atlasRenderMode,
                face = new FaceSpec
                {
                    PointSize = face.pointSize,
                    Scale = face.scale,
                    LineHeight = face.lineHeight,
                    AscentLine = face.ascentLine,
                    CapLine = face.capLine,
                    MeanLine = face.meanLine,
                    Baseline = face.baseline,
                    DescentLine = face.descentLine,
                    SuperscriptOffset = face.superscriptOffset,
                    SuperscriptSize = face.superscriptSize,
                    SubscriptOffset = face.subscriptOffset,
                    SubscriptSize = face.subscriptSize,
                    UnderlineOffset = face.underlineOffset,
                    UnderlineThickness = face.underlineThickness,
                    StrikethroughOffset = face.strikethroughOffset,
                    StrikethroughThickness = face.strikethroughThickness,
                    TabWidth = face.tabWidth,
                },
                properties = new PropertySpec
                {
                    normalStyle = asset.normalStyle,
                    normalSpacingOffset = asset.normalSpacingOffset,
                    boldStyle = asset.boldStyle,
                    boldSpacing = asset.boldSpacing,
                    italicStyle = asset.italicStyle,
                    tabSize = asset.tabSize,
                },
                floats = GetProperties(material, ShaderPropertyType.Float, ShaderPropertyType.Range)
                    .Select(name => new NamedFloat { name = name, value = material.GetFloat(name) })
                    .ToArray(),
                colors = GetProperties(material, ShaderPropertyType.Color)
                    .Select(name =>
                    {
                        var color = material.GetColor(name);
                        return new NamedColor
                        {
                            name = name,
                            r = color.r,
                            g = color.g,
                            b = color.b,
                            a = color.a,
                        };
                    })
                    .ToArray(),
                fallbacks = asset
                    .fallbackFontAssetTable.Where(fallback => fallback != null)
                    .Select(fallback => fallback.name)
                    .ToArray(),
                characters = characters.ToArray(),
                kerning = ExportKerning(asset, unicodeOf),
                atlases = files.ToArray(),
            };
        }

        private static IEnumerable<string> GetProperties(
            Material material,
            params ShaderPropertyType[] types
        )
        {
            var shader = material.shader;
            return Enumerable
                .Range(0, shader.GetPropertyCount())
                .Where(index => types.Contains(shader.GetPropertyType(index)))
                .Select(index => shader.GetPropertyName(index))
                .OrderBy(name => name, System.StringComparer.Ordinal);
        }

        private static KerningSpec[] ExportKerning(
            TMP_FontAsset asset,
            IReadOnlyDictionary<uint, uint> unicodeOf
        )
        {
            var pairs = new List<KerningSpec>();
            foreach (var record in asset.fontFeatureTable.glyphPairAdjustmentRecords)
            {
                var first = record.firstAdjustmentRecord;
                var second = record.secondAdjustmentRecord;
                if (
                    !unicodeOf.TryGetValue(first.glyphIndex, out var firstUnicode)
                    || !unicodeOf.TryGetValue(second.glyphIndex, out var secondUnicode)
                )
                {
                    continue;
                }
                pairs.Add(
                    new KerningSpec
                    {
                        first = firstUnicode,
                        second = secondUnicode,
                        firstAdjustment = ExportAdjustment(first.glyphValueRecord),
                        secondAdjustment = ExportAdjustment(second.glyphValueRecord),
                    }
                );
            }
            return pairs.OrderBy(pair => pair.first).ThenBy(pair => pair.second).ToArray();
        }

        private static AdjustmentSpec ExportAdjustment(TMP_GlyphValueRecord value)
        {
            return new AdjustmentSpec
            {
                xPlacement = value.xPlacement,
                yPlacement = value.yPlacement,
                xAdvance = value.xAdvance,
                yAdvance = value.yAdvance,
            };
        }

        private static TextAssetsSpec ExportTextAssets(Setup setup)
        {
            var sprites = AssetDatabase.LoadAssetAtPath<TMP_SpriteAsset>(
                SpriteAssetBuilder.GetAssetPath(setup.sprites)
            );
            return new TextAssetsSpec
            {
                fonts = Resources
                    .LoadAll<TMP_FontAsset>("Fonts & Materials")
                    .Select(font => font.name)
                    .Distinct()
                    .OrderBy(name => name, System.StringComparer.Ordinal)
                    .ToArray(),

                materials = Resources
                    .LoadAll<Material>("Fonts & Materials")
                    .Select(material =>
                        Path.GetFileNameWithoutExtension(AssetDatabase.GetAssetPath(material))
                    )
                    .Distinct()
                    .OrderBy(name => name, System.StringComparer.Ordinal)
                    .ToArray(),
                sprites = sprites
                    .spriteCharacterTable.Select(character => character.name)
                    .ToArray(),
                styles = TmpSetup
                    .CreateDefaultStyles()
                    .Select(style => new StyleSpec
                    {
                        name = style.name,
                        openMarkup = style.styleOpeningDefinition,
                        closeMarkup = style.styleClosingDefinition,
                    })
                    .ToArray(),
            };
        }
    }
}
