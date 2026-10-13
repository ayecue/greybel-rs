using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using TMPro;
using UnityEngine;
using UnityEngine.TextCore.LowLevel;

namespace TextMeshReference.Editor
{
    public static class FontRecorder
    {
        [Serializable]
        private sealed class RecordedFont
        {
            public string name;
            public string source;
            public int pointSize;
            public int padding;
            public int atlasWidth;
            public int atlasHeight;
            public int renderMode;
            public RecordedFace face;
            public List<RecordedGlyph> glyphs = new List<RecordedGlyph>();
            public List<RecordedPair> kerning = new List<RecordedPair>();
            public List<string> atlases = new List<string>();
        }

        [Serializable]
        private sealed class RecordedFace
        {
            public string familyName;
            public string styleName;
            public float pointSize;
            public float scale;
            public float lineHeight;
            public float ascentLine;
            public float capLine;
            public float meanLine;
            public float baseline;
            public float descentLine;
            public float superscriptOffset;
            public float superscriptSize;
            public float subscriptOffset;
            public float subscriptSize;
            public float underlineOffset;
            public float underlineThickness;
            public float strikethroughOffset;
            public float strikethroughThickness;
            public float tabWidth;
        }

        [Serializable]
        private sealed class RecordedGlyph
        {
            public uint unicode;
            public uint index;
            public float width;
            public float height;
            public float bearingX;
            public float bearingY;
            public float advance;
            public int x;
            public int y;
            public int rectWidth;
            public int rectHeight;
            public int atlas;
        }

        [Serializable]
        private sealed class RecordedPair
        {
            public uint first;
            public uint second;
            public float firstXPlacement;
            public float firstYPlacement;
            public float firstXAdvance;
            public float firstYAdvance;
            public float secondXPlacement;
            public float secondYPlacement;
            public float secondXAdvance;
            public float secondYAdvance;
        }

        public static void Run()
        {
            var setup = Setup.FromCommandLine();
            var output = CommandLine.GetRequiredArgument("-recordOutput");
            Directory.CreateDirectory(output);
            var specsToRecord = setup.fonts.Where(spec => spec.IsGiven).Concat(setup.bundled);
            foreach (var spec in specsToRecord.Where(spec => !string.IsNullOrEmpty(spec.source)))
            {
                Record(setup, spec, output);
            }
        }

        private static void Record(Setup setup, FontSpec spec, string output)
        {
            var font = FontAssetBuilder.Import(setup, spec);
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
            asset.TryAddCharacters(spec.GetUnicodes(), out _);
            var face = asset.faceInfo;
            var recordedFont = new RecordedFont
            {
                name = spec.name,
                source = spec.source,
                pointSize = spec.pointSize,
                padding = spec.padding,
                atlasWidth = spec.atlasWidth,
                atlasHeight = spec.atlasHeight,
                renderMode = spec.renderMode,
                face = new RecordedFace
                {
                    familyName = face.familyName,
                    styleName = face.styleName,
                    pointSize = face.pointSize,
                    scale = face.scale,
                    lineHeight = face.lineHeight,
                    ascentLine = face.ascentLine,
                    capLine = face.capLine,
                    meanLine = face.meanLine,
                    baseline = face.baseline,
                    descentLine = face.descentLine,
                    superscriptOffset = face.superscriptOffset,
                    superscriptSize = face.superscriptSize,
                    subscriptOffset = face.subscriptOffset,
                    subscriptSize = face.subscriptSize,
                    underlineOffset = face.underlineOffset,
                    underlineThickness = face.underlineThickness,
                    strikethroughOffset = face.strikethroughOffset,
                    strikethroughThickness = face.strikethroughThickness,
                    tabWidth = face.tabWidth,
                },
            };
            var unicodeOf = new Dictionary<uint, uint>();
            foreach (var character in asset.characterTable.OrderBy(character => character.unicode))
            {
                var glyph = character.glyph;
                unicodeOf[glyph.index] = character.unicode;
                recordedFont.glyphs.Add(
                    new RecordedGlyph
                    {
                        unicode = character.unicode,
                        index = glyph.index,
                        width = glyph.metrics.width,
                        height = glyph.metrics.height,
                        bearingX = glyph.metrics.horizontalBearingX,
                        bearingY = glyph.metrics.horizontalBearingY,
                        advance = glyph.metrics.horizontalAdvance,
                        x = glyph.glyphRect.x,
                        y = glyph.glyphRect.y,
                        rectWidth = glyph.glyphRect.width,
                        rectHeight = glyph.glyphRect.height,
                        atlas = glyph.atlasIndex,
                    }
                );
            }
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
                recordedFont.kerning.Add(
                    new RecordedPair
                    {
                        first = firstUnicode,
                        second = secondUnicode,
                        firstXPlacement = first.glyphValueRecord.xPlacement,
                        firstYPlacement = first.glyphValueRecord.yPlacement,
                        firstXAdvance = first.glyphValueRecord.xAdvance,
                        firstYAdvance = first.glyphValueRecord.yAdvance,
                        secondXPlacement = second.glyphValueRecord.xPlacement,
                        secondYPlacement = second.glyphValueRecord.yPlacement,
                        secondXAdvance = second.glyphValueRecord.xAdvance,
                        secondYAdvance = second.glyphValueRecord.yAdvance,
                    }
                );
            }
            recordedFont.kerning = recordedFont
                .kerning.OrderBy(pair => pair.first)
                .ThenBy(pair => pair.second)
                .ToList();
            for (
                var index = 0;
                index < asset.atlasTextures.Length && asset.atlasTextures[index] != null;
                index++
            )
            {
                var file = $"{spec.name}.{index}.png";
                File.WriteAllBytes(
                    Path.Combine(output, file),
                    AtlasPng.ReadAlphaPng(asset.atlasTextures[index])
                );
                recordedFont.atlases.Add(file);
            }
            File.WriteAllText(
                Path.Combine(output, spec.name + ".json"),
                JsonUtility.ToJson(recordedFont, true)
            );
            UnityEngine.Object.DestroyImmediate(asset);
        }
    }
}
