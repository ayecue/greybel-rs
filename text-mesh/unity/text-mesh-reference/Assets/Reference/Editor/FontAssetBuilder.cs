using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using TMPro;
using UnityEditor;
using UnityEngine;
using UnityEngine.TextCore;
using UnityEngine.TextCore.LowLevel;

namespace TextMeshReference.Editor
{
    public static class FontAssetBuilder
    {
        public const string FontFolder = "Assets/Generated/Fonts";
        public const string AssetFolder = "Assets/Generated/Resources/Fonts & Materials";
        private const float Tolerance = 0.01f;
        private const int MaxAtlasScale = 8;
        private static readonly string[] AtlasProperties = { "_TextureWidth", "_TextureHeight" };

        public static Dictionary<string, TMP_FontAsset> Build(Setup setup, StringBuilder report)
        {
            Directory.CreateDirectory(FontFolder);
            Directory.CreateDirectory(AssetFolder);
            var builtAssets = new Dictionary<string, TMP_FontAsset>();
            foreach (var spec in setup.bundled)
            {
                var asset = Resources.Load<TMP_FontAsset>("Fonts & Materials/" + spec.name);
                if (asset == null)
                {
                    report.AppendLine($"{spec.name}: missing from TMP Essential Resources");
                    continue;
                }
                builtAssets[spec.name] = asset;
            }
            foreach (var spec in setup.fonts)
            {
                builtAssets[spec.name] = Bake(setup, spec);
            }
            foreach (var spec in setup.fonts)
            {
                var asset = builtAssets[spec.name];
                asset.fallbackFontAssetTable = spec
                    .fallbacks.Where(builtAssets.ContainsKey)
                    .Select(name => builtAssets[name])
                    .ToList();
                EditorUtility.SetDirty(asset);
            }
            AssetDatabase.SaveAssets();
            foreach (
                var spec in setup
                    .fonts.Concat(setup.bundled)
                    .Where(spec => spec.IsGiven && builtAssets.ContainsKey(spec.name))
            )
            {
                Compare(spec, builtAssets[spec.name], report);
                ApplyGivenMetrics(spec, builtAssets[spec.name]);
                ApplyGivenAtlases(setup, spec, builtAssets[spec.name], report);
            }
            AssetDatabase.SaveAssets();
            return builtAssets;
        }

        public static Font Import(Setup setup, FontSpec spec)
        {
            Directory.CreateDirectory(FontFolder);
            var fontPath = $"{FontFolder}/{spec.source}";
            File.Copy(setup.GetFontFile(spec.source), fontPath, true);
            AssetDatabase.ImportAsset(fontPath);
            return AssetDatabase.LoadAssetAtPath<Font>(fontPath);
        }

        private static TMP_FontAsset Bake(Setup setup, FontSpec spec)
        {
            var font = Import(setup, spec);
            var assetPath = $"{AssetFolder}/{spec.name}.asset";
            AssetDatabase.DeleteAsset(assetPath);
            var unicodes = spec.GetUnicodes();
            var asset = spec.IsGiven
                ? BakeToFit(spec, font, unicodes)
                : BakeSpread(spec, font, unicodes);
            asset.name = spec.name;
            asset.atlasPopulationMode = AtlasPopulationMode.Static;

            if (spec.IsGiven)
            {
                asset.normalStyle = spec.properties.normalStyle;
                asset.normalSpacingOffset = spec.properties.normalSpacingOffset;
                asset.boldStyle = spec.properties.boldStyle;
                asset.boldSpacing = spec.properties.boldSpacing;
                asset.italicStyle = spec.properties.italicStyle;
                asset.tabSize = spec.properties.tabSize;
            }
            AssetDatabase.CreateAsset(asset, assetPath);
            for (
                var index = 0;
                index < asset.atlasTextures.Length && asset.atlasTextures[index] != null;
                index++
            )
            {
                var atlas = asset.atlasTextures[index];
                atlas.name = index == 0 ? spec.name + " Atlas" : $"{spec.name} Atlas {index}";
                AssetDatabase.AddObjectToAsset(atlas, asset);
            }
            var material = asset.material;
            material.name = spec.name + " Material";
            foreach (var value in spec.floats ?? new NamedFloat[0])
            {
                if (material.HasProperty(value.name) && !AtlasProperties.Contains(value.name))
                {
                    material.SetFloat(value.name, value.value);
                }
            }
            foreach (var color in spec.colors ?? new NamedColor[0])
            {
                if (material.HasProperty(color.name))
                {
                    material.SetColor(color.name, new Color(color.r, color.g, color.b, color.a));
                }
            }
            AssetDatabase.AddObjectToAsset(material, asset);
            EditorUtility.SetDirty(asset);
            return asset;
        }

        private static TMP_FontAsset BakeSpread(FontSpec spec, Font font, uint[] unicodes)
        {
            var asset = TMP_FontAsset.CreateFontAsset(
                font,
                spec.pointSize,
                spec.padding,
                (GlyphRenderMode)spec.renderMode,
                spec.atlasWidth,
                spec.atlasHeight,
                AtlasPopulationMode.Dynamic,
                true
            );
            asset.TryAddCharacters(unicodes, out _);
            return asset;
        }

        private static TMP_FontAsset BakeToFit(FontSpec spec, Font font, uint[] unicodes)
        {
            TMP_FontAsset asset = null;
            var missingCount = int.MaxValue;
            for (var scale = 1; scale <= MaxAtlasScale; scale *= 2)
            {
                var candidate = TMP_FontAsset.CreateFontAsset(
                    font,
                    spec.pointSize,
                    spec.padding,
                    (GlyphRenderMode)spec.renderMode,
                    spec.atlasWidth * scale,
                    spec.atlasHeight * scale,
                    AtlasPopulationMode.Dynamic,
                    false
                );
                candidate.TryAddCharacters(unicodes, out var missingUnicodes);
                var count = missingUnicodes?.Length ?? 0;
                if (count >= missingCount)
                {
                    Object.DestroyImmediate(candidate);
                    break;
                }
                if (asset != null)
                {
                    Object.DestroyImmediate(asset);
                }
                asset = candidate;
                missingCount = count;
                if (missingCount == 0)
                {
                    break;
                }
            }
            return asset;
        }

        private static void ApplyGivenMetrics(FontSpec spec, TMP_FontAsset asset)
        {
            var face = asset.faceInfo;
            face.pointSize = (int)spec.face.PointSize;
            face.scale = spec.face.Scale;
            face.lineHeight = spec.face.LineHeight;
            face.ascentLine = spec.face.AscentLine;
            face.capLine = spec.face.CapLine;
            face.meanLine = spec.face.MeanLine;
            face.baseline = spec.face.Baseline;
            face.descentLine = spec.face.DescentLine;
            face.superscriptOffset = spec.face.SuperscriptOffset;
            face.superscriptSize = spec.face.SuperscriptSize;
            face.subscriptOffset = spec.face.SubscriptOffset;
            face.subscriptSize = spec.face.SubscriptSize;
            face.underlineOffset = spec.face.UnderlineOffset;
            face.underlineThickness = spec.face.UnderlineThickness;
            face.strikethroughOffset = spec.face.StrikethroughOffset;
            face.strikethroughThickness = spec.face.StrikethroughThickness;
            face.tabWidth = spec.face.TabWidth;
            asset.faceInfo = face;
            foreach (var character in spec.characters)
            {
                if (asset.characterLookupTable.TryGetValue(character.unicode, out var ours))
                {
                    ours.glyph.metrics = new GlyphMetrics(
                        character.width,
                        character.height,
                        character.bearingX,
                        character.bearingY,
                        character.advance
                    );
                }
            }
            EditorUtility.SetDirty(asset);
        }

        private static void ApplyGivenAtlases(
            Setup setup,
            FontSpec spec,
            TMP_FontAsset asset,
            StringBuilder report
        )
        {
            var textures = spec
                .atlases.Select((file, index) => LoadAtlas(setup.GetAtlasFile(file), spec, index))
                .ToArray();
            var previousAtlases = asset.atlasTextures.Where(texture => texture != null).ToArray();
            RetargetPresetMaterials(previousAtlases, textures[0], spec);
            foreach (var texture in previousAtlases)
            {
                if (AssetDatabase.IsSubAsset(texture))
                {
                    AssetDatabase.RemoveObjectFromAsset(texture);
                    Object.DestroyImmediate(texture, true);
                }
            }
            foreach (var texture in textures)
            {
                AssetDatabase.AddObjectToAsset(texture, asset);
            }
            var serializedAsset = new SerializedObject(asset);
            serializedAsset.FindProperty("m_AtlasWidth").intValue = spec.atlasWidth;
            serializedAsset.FindProperty("m_AtlasHeight").intValue = spec.atlasHeight;
            serializedAsset.FindProperty("m_AtlasPadding").intValue = spec.padding;
            serializedAsset.FindProperty("m_AtlasTextureIndex").intValue = textures.Length - 1;
            var array = serializedAsset.FindProperty("m_AtlasTextures");
            array.arraySize = textures.Length;
            for (var index = 0; index < textures.Length; index++)
            {
                array.GetArrayElementAtIndex(index).objectReferenceValue = textures[index];
            }
            serializedAsset.ApplyModifiedPropertiesWithoutUndo();
            typeof(TMP_FontAsset)
                .GetField(
                    "m_AtlasTexture",
                    System.Reflection.BindingFlags.Instance
                        | System.Reflection.BindingFlags.NonPublic
                )
                ?.SetValue(asset, null);
            var unmatchedCount = 0;
            foreach (var character in spec.characters)
            {
                if (!asset.characterLookupTable.TryGetValue(character.unicode, out var ours))
                {
                    continue;
                }

                var outside =
                    character.x < 0
                    || character.y < 0
                    || character.rectWidth < 0
                    || character.rectHeight < 0;
                ours.glyph.glyphRect = outside
                    ? new GlyphRect(0, 0, 0, 0)
                    : new GlyphRect(
                        character.x,
                        character.y,
                        character.rectWidth,
                        character.rectHeight
                    );
                ours.glyph.atlasIndex = character.atlas;
            }
            foreach (var glyph in asset.glyphTable)
            {
                if (
                    !spec.characters.Any(character =>
                        asset.characterLookupTable.TryGetValue(character.unicode, out var ours)
                        && ours.glyph == glyph
                    )
                )
                {
                    unmatchedCount++;
                }
            }
            var material = asset.material;
            material.SetTexture(ShaderUtilities.ID_MainTex, textures[0]);
            material.SetFloat(ShaderUtilities.ID_TextureWidth, spec.atlasWidth);
            material.SetFloat(ShaderUtilities.ID_TextureHeight, spec.atlasHeight);
            EditorUtility.SetDirty(material);
            EditorUtility.SetDirty(asset);
            report.AppendLine(
                $"{spec.name}: given atlases {textures.Length} ({spec.atlasWidth}x{spec.atlasHeight}), "
                    + $"{unmatchedCount} glyphs without a given rectangle"
            );
        }

        private static void RetargetPresetMaterials(
            Texture2D[] previousAtlases,
            Texture2D atlas,
            FontSpec spec
        )
        {
            foreach (var guid in AssetDatabase.FindAssets("t:Material"))
            {
                var material = AssetDatabase.LoadAssetAtPath<Material>(
                    AssetDatabase.GUIDToAssetPath(guid)
                );
                if (
                    material == null
                    || !material.HasProperty(ShaderUtilities.ID_MainTex)
                    || !previousAtlases.Contains(material.GetTexture(ShaderUtilities.ID_MainTex))
                )
                {
                    continue;
                }
                material.SetTexture(ShaderUtilities.ID_MainTex, atlas);
                material.SetFloat(ShaderUtilities.ID_TextureWidth, spec.atlasWidth);
                material.SetFloat(ShaderUtilities.ID_TextureHeight, spec.atlasHeight);
                EditorUtility.SetDirty(material);
            }
        }

        private static Texture2D LoadAtlas(string path, FontSpec spec, int index)
        {
            var decodedTexture = new Texture2D(2, 2, TextureFormat.RGBA32, false);
            decodedTexture.LoadImage(File.ReadAllBytes(path));
            if (
                decodedTexture.width != spec.atlasWidth
                || decodedTexture.height != spec.atlasHeight
            )
            {
                throw new InvalidDataException(
                    $"{path}: {decodedTexture.width}x{decodedTexture.height}, the font's atlas is {spec.atlasWidth}x{spec.atlasHeight}"
                );
            }
            var alpha = decodedTexture.GetPixels32().Select(pixel => pixel.r).ToArray();
            Object.DestroyImmediate(decodedTexture);
            var texture = new Texture2D(
                spec.atlasWidth,
                spec.atlasHeight,
                TextureFormat.Alpha8,
                false
            )
            {
                name = index == 0 ? spec.name + " Atlas" : $"{spec.name} Atlas {index}",
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
            };
            texture.SetPixelData(alpha, 0);
            texture.Apply(false, false);
            return texture;
        }

        private static void Compare(FontSpec spec, TMP_FontAsset asset, StringBuilder report)
        {
            var face = asset.faceInfo;
            var differences = new List<string>();
            var largest = new Dictionary<string, float>();
            void Check(string what, float givenValue, float bakedValue)
            {
                var difference = Mathf.Abs(givenValue - bakedValue);
                if (difference <= Tolerance)
                {
                    return;
                }
                differences.Add($"{what}: given {givenValue}, baked {bakedValue}");
                var kind = what.StartsWith("U+") ? what.Substring(what.IndexOf(' ') + 1) : what;
                largest[kind] = Mathf.Max(
                    largest.TryGetValue(kind, out var previousLargest) ? previousLargest : 0f,
                    difference
                );
            }
            Check("point size", spec.face.PointSize, face.pointSize);
            Check("line height", spec.face.LineHeight, face.lineHeight);
            Check("ascent", spec.face.AscentLine, face.ascentLine);
            Check("descent", spec.face.DescentLine, face.descentLine);
            Check("underline offset", spec.face.UnderlineOffset, face.underlineOffset);
            Check("strikethrough offset", spec.face.StrikethroughOffset, face.strikethroughOffset);
            var missingCharacters = new List<string>();
            foreach (var character in spec.characters)
            {
                if (!asset.characterLookupTable.TryGetValue(character.unicode, out var ours))
                {
                    missingCharacters.Add($"U+{character.unicode:X4}");
                    continue;
                }
                var metrics = ours.glyph.metrics;
                var label = $"U+{character.unicode:X4}";
                Check(label + " advance", character.advance, metrics.horizontalAdvance);
                if (metrics.width == 0f)
                {
                    continue;
                }
                Check(label + " bearing x", character.bearingX, metrics.horizontalBearingX);
                Check(label + " bearing y", character.bearingY, metrics.horizontalBearingY);
                Check(label + " width", character.width, metrics.width);
                Check(label + " height", character.height, metrics.height);
            }
            report.AppendLine(
                $"{spec.name}: {spec.characters.Length - missingCharacters.Count}/{spec.characters.Length} characters, "
                    + $"{differences.Count} differences"
            );
            if (missingCharacters.Count > 0)
            {
                report.AppendLine("  missing: " + string.Join(" ", missingCharacters));
            }
            foreach (var pair in largest.OrderByDescending(pair => pair.Value))
            {
                report.AppendLine($"  {pair.Key}: up to {pair.Value:0.###} px");
            }
        }
    }
}
