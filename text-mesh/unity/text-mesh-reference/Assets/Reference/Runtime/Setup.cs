using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEngine;

namespace TextMeshReference
{
    [Serializable]
    public sealed class Setup
    {
        public TextSettings text;
        public FrameSpec frame;
        public ThemeSpec theme;
        public SpriteAssetSpec sprites;
        public string terminal;
        public string[] globalFallbacks;
        public int missingGlyphCharacter;
        public FontSpec[] fonts;
        public FontSpec[] bundled;
        public TextAssetsSpec textAssets;

        [NonSerialized]
        public string Folder;

        public static Setup Load(string path)
        {
            var setup = JsonUtility.FromJson<Setup>(File.ReadAllText(path));
            setup.Folder = Path.GetDirectoryName(Path.GetFullPath(path));
            return setup;
        }

        public static Setup FromCommandLine()
        {
            return Load(CommandLine.GetRequiredArgument("-setup"));
        }

        public string GetFontFile(string source) => Path.Combine(Folder, "fonts", source);

        public string GetAtlasFile(string file) => Path.Combine(Folder, "atlases", file);

        public string GetSpriteSheetFile(string file) => Path.Combine(Folder, "sprites", file);
    }

    [Serializable]
    public sealed class FontSpec
    {
        public string name;
        public string source;
        public string family;
        public string style;
        public int pointSize;
        public int padding;
        public int atlasWidth;
        public int atlasHeight;
        public int renderMode;

        public string characterRanges;
        public FaceSpec face;
        public PropertySpec properties;
        public NamedFloat[] floats;
        public NamedColor[] colors;
        public string[] fallbacks;
        public CharacterSpec[] characters;
        public KerningSpec[] kerning;
        public string[] atlases;

        public bool IsGiven => atlases != null && atlases.Length > 0;

        public uint[] GetUnicodes()
        {
            if (characters != null && characters.Length > 0)
            {
                return characters.Select(character => character.unicode).ToArray();
            }
            var unicodes = new List<uint>();
            foreach (
                var range in (characterRanges ?? "").Split(
                    new[] { ',' },
                    StringSplitOptions.RemoveEmptyEntries
                )
            )
            {
                var bounds = range.Trim().Split('-');
                var first = Convert.ToUInt32(bounds[0], 16);
                var last = bounds.Length > 1 ? Convert.ToUInt32(bounds[1], 16) : first;
                for (var unicode = first; unicode <= last; unicode++)
                {
                    unicodes.Add(unicode);
                }
            }
            return unicodes.ToArray();
        }
    }

    [Serializable]
    public sealed class FaceSpec
    {
        public float PointSize;
        public float Scale;
        public float LineHeight;
        public float AscentLine;
        public float CapLine;
        public float MeanLine;
        public float Baseline;
        public float DescentLine;
        public float SuperscriptOffset;
        public float SuperscriptSize;
        public float SubscriptOffset;
        public float SubscriptSize;
        public float UnderlineOffset;
        public float UnderlineThickness;
        public float StrikethroughOffset;
        public float StrikethroughThickness;
        public float TabWidth;
    }

    [Serializable]
    public sealed class PropertySpec
    {
        public float normalStyle;
        public float normalSpacingOffset;
        public float boldStyle;
        public float boldSpacing;
        public byte italicStyle;
        public byte tabSize;
    }

    [Serializable]
    public sealed class NamedFloat
    {
        public string name;
        public float value;
    }

    [Serializable]
    public sealed class NamedColor
    {
        public string name;
        public float r;
        public float g;
        public float b;
        public float a;
    }

    [Serializable]
    public sealed class CharacterSpec
    {
        public uint unicode;
        public float advance;
        public float bearingX;
        public float bearingY;
        public float width;
        public float height;
        public int x;
        public int y;
        public int rectWidth;
        public int rectHeight;
        public int atlas;
    }

    [Serializable]
    public sealed class KerningSpec
    {
        public uint first;
        public uint second;
        public AdjustmentSpec firstAdjustment;
        public AdjustmentSpec secondAdjustment;
    }

    [Serializable]
    public sealed class AdjustmentSpec
    {
        public float xPlacement;
        public float yPlacement;
        public float xAdvance;
        public float yAdvance;
    }

    [Serializable]
    public sealed class SpriteAssetSpec
    {
        public string name;
        public string sheet;
        public int sheetWidth;
        public int sheetHeight;
        public FaceSpec face;
        public SpriteGlyphSpec[] glyphs;
        public SpriteCharacterSpec[] characters;
    }

    [Serializable]
    public sealed class SpriteGlyphSpec
    {
        public int index;
        public int x;
        public int y;
        public int rectWidth;
        public int rectHeight;
        public float advance;
        public float bearingX;
        public float bearingY;
        public float width;
        public float height;
        public float scale;
    }

    [Serializable]
    public sealed class SpriteCharacterSpec
    {
        public string name;
        public uint unicode;
        public int glyph;
        public float scale;
    }

    [Serializable]
    public sealed class FrameSpec
    {
        public float padding;
        public float itemGap;
        public float itemMinHeight;
    }

    [Serializable]
    public sealed class ThemeSpec
    {
        public ColorSpec text;
        public ColorSpec background;
    }

    [Serializable]
    public sealed class ColorSpec
    {
        public byte r;
        public byte g;
        public byte b;
        public byte a;

        public Color32 ToColor32() => new Color32(r, g, b, a);
    }

    [Serializable]
    public sealed class TextAssetsSpec
    {
        public string[] fonts;
        public string[] materials;
        public string[] sprites;
        public StyleSpec[] styles;
    }

    [Serializable]
    public sealed class StyleSpec
    {
        public string name;
        public string openMarkup;
        public string closeMarkup;
    }
}
