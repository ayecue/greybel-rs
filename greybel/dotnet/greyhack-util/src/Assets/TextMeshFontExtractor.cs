using System.Text.Json.Nodes;
using AssetsTools.NET;
using AssetsTools.NET.Extra;
using static GreyHackUtil.Assets.AssetFieldReader;

namespace GreyHackUtil.Assets;

/// <summary>
/// Extracts terminal TextMeshPro fonts into the text mesh bundle and adds each font specification and its atlases to
/// <paramref name="files"/>.
/// </summary>
internal sealed class TextMeshFontExtractor(
    GameData game,
    List<(string Name, byte[] Contents)> files
)
{
    private static readonly string[] FontProperties =
    [
        "normalStyle",
        "normalSpacingOffset",
        "boldStyle",
        "boldSpacing",
        "italicStyle",
        "tabSize",
    ];

    private const string ExcludedFallbackPrefix = "NotoSansMonoCJK";

    /// <summary>Maps text mesh bundle keys to the terminal line font, its fallbacks, and global fallbacks.</summary>
    public IEnumerable<(string Key, JsonNode Value)> Extract(GameAsset text)
    {
        GameAsset settings = game.GetAssetsOfClass("TMP_Settings").First();
        var order = new List<string>();
        var specs = new Dictionary<string, JsonObject>();

        void Visit(GameAsset font)
        {
            string name = font.Name;
            if (
                specs.ContainsKey(name)
                || name.StartsWith(ExcludedFallbackPrefix, StringComparison.Ordinal)
            )
            {
                return;
            }
            specs[name] = ExtractFontSpec(font);
            order.Add(name);
            foreach (var fallback in GetArray(font.Fields, "m_FallbackFontAssetTable"))
            {
                Visit(Require(game.Dereference(font, fallback), "a fallback font"));
            }
        }

        Visit(Require(game.Dereference(text, text.Fields["m_fontAsset"]), "the line's font"));
        var globalFallbacks = new JsonArray();
        foreach (var pointer in GetArray(settings.Fields, "m_fallbackFontAssets"))
        {
            GameAsset font = Require(game.Dereference(settings, pointer), "a global fallback font");
            if (!font.Name.StartsWith(ExcludedFallbackPrefix, StringComparison.Ordinal))
            {
                globalFallbacks.Add(font.Name);
                Visit(font);
            }
        }
        var bundledFonts = new List<JsonObject>();
        foreach (var (path, asset) in game.Resources)
        {
            if (
                asset == null
                || !path.StartsWith("fonts & materials/", StringComparison.Ordinal)
                || game.GetClass(asset) != "TMP_FontAsset"
            )
            {
                continue;
            }
            GameAsset font = game.GetAssetsOfClass("TMP_FontAsset")
                .First(candidate =>
                    candidate.File.name == asset.File.name
                    && candidate.Info.PathId == asset.Info.PathId
                );
            if (
                !specs.ContainsKey(font.Name)
                && bundledFonts.All(spec => (string?)spec["name"] != font.Name)
            )
            {
                bundledFonts.Add(ExtractFontSpec(font));
            }
        }
        yield return ("terminal", order[0]);
        yield return (
            "bundled",
            new JsonArray(bundledFonts.Select(spec => (JsonNode?)spec).ToArray())
        );
        yield return ("globalFallbacks", globalFallbacks);
        yield return (
            "missingGlyphCharacter",
            ExtractNumber(settings.Fields["m_missingGlyphCharacter"])!
        );
        yield return (
            "fonts",
            new JsonArray(order.Select(name => (JsonNode?)specs[name]).ToArray())
        );
    }

    private JsonObject ExtractFontSpec(GameAsset font)
    {
        AssetTypeValueField tree = font.Fields;
        AssetTypeValueField face = tree["m_FaceInfo"];
        GameAsset? material = tree["material"].IsDummy
            ? null
            : game.Dereference(font, tree["material"]);
        AssetTypeValueField? savedProperties = material?.Fields["m_SavedProperties"];
        var glyphs = tree["m_GlyphTable.Array"]
            .Children.ToDictionary(glyph => glyph["m_Index"].AsUInt);
        var unicodes = new Dictionary<uint, List<uint>>();
        foreach (var character in tree["m_CharacterTable.Array"].Children)
        {
            uint glyph = character["m_GlyphIndex"].AsUInt;
            if (!unicodes.TryGetValue(glyph, out var list))
            {
                unicodes[glyph] = list = [];
            }
            list.Add(character["m_Unicode"].AsUInt);
        }
        var kerning = new JsonArray();
        AssetTypeValueField features = tree["m_FontFeatureTable"];
        foreach (
            var record in features.IsDummy ? [] : GetArray(features, "m_GlyphPairAdjustmentRecords")
        )
        {
            var first = record["m_FirstAdjustmentRecord"];
            var second = record["m_SecondAdjustmentRecord"];
            foreach (uint left in unicodes.GetValueOrDefault(first["m_GlyphIndex"].AsUInt) ?? [])
            {
                foreach (
                    uint right in unicodes.GetValueOrDefault(second["m_GlyphIndex"].AsUInt) ?? []
                )
                {
                    kerning.Add(
                        new JsonObject
                        {
                            ["first"] = left,
                            ["second"] = right,
                            ["firstAdjustment"] = ExtractAdjustment(first["m_GlyphValueRecord"]),
                            ["secondAdjustment"] = ExtractAdjustment(second["m_GlyphValueRecord"]),
                        }
                    );
                }
            }
        }
        var characters = new JsonArray();
        foreach (var character in tree["m_CharacterTable.Array"].Children)
        {
            var glyph = glyphs[character["m_GlyphIndex"].AsUInt];
            var metrics = glyph["m_Metrics"];
            var rect = glyph["m_GlyphRect"];
            characters.Add(
                new JsonObject
                {
                    ["unicode"] = ExtractNumber(character["m_Unicode"]),
                    ["advance"] = ExtractNumber(metrics["m_HorizontalAdvance"]),
                    ["bearingX"] = ExtractNumber(metrics["m_HorizontalBearingX"]),
                    ["bearingY"] = ExtractNumber(metrics["m_HorizontalBearingY"]),
                    ["width"] = ExtractNumber(metrics["m_Width"]),
                    ["height"] = ExtractNumber(metrics["m_Height"]),
                    ["x"] = ExtractNumber(rect["m_X"]),
                    ["y"] = ExtractNumber(rect["m_Y"]),
                    ["rectWidth"] = ExtractNumber(rect["m_Width"]),
                    ["rectHeight"] = ExtractNumber(rect["m_Height"]),
                    ["atlas"] = ExtractNumber(glyph["m_AtlasIndex"]),
                }
            );
        }
        var properties = new JsonObject();
        foreach (string key in FontProperties.Where(key => !tree[key].IsDummy))
        {
            properties[key] = ExtractNumber(tree[key]);
        }
        string family = face["m_FamilyName"].AsString;
        return new JsonObject
        {
            ["name"] = font.Name,
            ["source"] = FontSourceCatalog.GetFile(family) ?? "",
            ["family"] = family,
            ["style"] = face["m_StyleName"].AsString,
            ["pointSize"] = ExtractNumber(face["m_PointSize"]),
            ["padding"] = ExtractNumber(tree["m_AtlasPadding"]),
            ["atlasWidth"] = ExtractNumber(tree["m_AtlasWidth"]),
            ["atlasHeight"] = ExtractNumber(tree["m_AtlasHeight"]),
            ["renderMode"] = ExtractNumber(tree["m_AtlasRenderMode"]),
            ["face"] = ExtractNumbers(face),
            ["properties"] = properties,
            ["floats"] = new JsonArray(
                (savedProperties == null ? [] : GetArray(savedProperties, "m_Floats"))
                    .Select(pair =>
                        (JsonNode?)
                            new JsonObject
                            {
                                ["name"] = pair["first"].AsString,
                                ["value"] = ExtractNumber(pair["second"]),
                            }
                    )
                    .ToArray()
            ),
            ["colors"] = new JsonArray(
                (savedProperties == null ? [] : GetArray(savedProperties, "m_Colors"))
                    .Select(pair =>
                        (JsonNode?)
                            new JsonObject
                            {
                                ["name"] = pair["first"].AsString,
                                ["r"] = ExtractNumber(pair["second"]["r"]),
                                ["g"] = ExtractNumber(pair["second"]["g"]),
                                ["b"] = ExtractNumber(pair["second"]["b"]),
                                ["a"] = ExtractNumber(pair["second"]["a"]),
                            }
                    )
                    .ToArray()
            ),
            ["fallbacks"] = new JsonArray(
                GetArray(tree, "m_FallbackFontAssetTable")
                    .Select(pointer =>
                        (JsonNode?)Require(game.Dereference(font, pointer), "a fallback font").Name
                    )
                    .ToArray()
            ),
            ["characters"] = characters,
            ["kerning"] = kerning,
            ["atlases"] = ExtractAtlases(font),
        };
    }

    private JsonArray ExtractAtlases(GameAsset font)
    {
        var names = new JsonArray();
        var textures = GetArray(font.Fields, "m_AtlasTextures").ToList();
        for (int index = 0; index < textures.Count; index++)
        {
            GameAsset? texture = game.Dereference(font, textures[index]);
            if (texture == null)
            {
                continue;
            }
            var (_, _, bgra) = game.ReadPixels(texture);
            files.Add(($"atlases/{font.Name}.{index}.alpha", ToAlpha(bgra)));
            names.Add($"{font.Name}.{index}.png");
        }
        return names;
    }

    private static JsonObject ExtractAdjustment(AssetTypeValueField record) =>
        new()
        {
            ["xPlacement"] = ExtractNumber(record["m_XPlacement"]),
            ["yPlacement"] = ExtractNumber(record["m_YPlacement"]),
            ["xAdvance"] = ExtractNumber(record["m_XAdvance"]),
            ["yAdvance"] = ExtractNumber(record["m_YAdvance"]),
        };

    private static byte[] ToAlpha(byte[] bgra)
    {
        byte[] alpha = new byte[bgra.Length / 4];
        for (int pixel = 0; pixel < alpha.Length; pixel++)
        {
            alpha[pixel] = bgra[pixel * 4 + 3];
        }
        return alpha;
    }
}
