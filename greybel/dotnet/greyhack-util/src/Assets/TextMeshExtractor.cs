using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using AssetsTools.NET;
using AssetsTools.NET.Extra;
using static GreyHackUtil.Assets.AssetFieldReader;

namespace GreyHackUtil.Assets;

internal sealed class TextMeshExtractor(GameData game)
{
    private static readonly string[] TextProperties =
    [
        "m_fontSize",
        "m_fontWeight",
        "m_HorizontalAlignment",
        "m_VerticalAlignment",
        "m_characterSpacing",
        "m_wordSpacing",
        "m_lineSpacing",
        "m_lineSpacingMax",
        "m_paragraphSpacing",
        "m_charWidthMaxAdj",
        "m_enableWordWrapping",
        "m_wordWrappingRatios",
        "m_enableKerning",
        "m_enableExtraPadding",
        "m_isRichText",
        "m_parseCtrlCharacters",
        "m_isOrthographic",
        "m_tintAllSprites",
        "m_overrideHtmlColors",
    ];

    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        IndentSize = 1,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    private readonly List<(string Name, byte[] Contents)> files = [];

    public List<(string Name, byte[] Contents)> Extract(string cache)
    {
        GameAsset text = FindTerminalText();
        AssetTypeValueField tree = text.Fields;
        var component = new JsonObject();
        foreach (string key in TextProperties)
        {
            component[key[2..]] = ExtractNumber(tree[key]);
        }
        component["margin"] = new JsonArray(
            new[] { "x", "y", "z", "w" }
                .Select(axis => ExtractNumber(tree["m_margin"][axis]))
                .ToArray()
        );
        GameAsset sprites = Require(
            game.Dereference(text, tree["m_spriteAsset"]),
            "the line's sprite asset"
        );
        var setup = new JsonObject
        {
            ["text"] = component,
            ["frame"] = ExtractFrame(),
            ["theme"] = ExtractTheme(),
            ["sprites"] = ExtractSprites(sprites),
        };
        foreach (var (key, value) in new TextMeshFontExtractor(game, files).Extract(text))
        {
            setup[key] = value;
        }
        setup["textAssets"] = ExtractTextAssets(sprites);
        files.Add(
            (
                "text-mesh.json",
                JsonSerializer
                    .SerializeToUtf8Bytes(setup, Options)
                    .Concat("\n"u8.ToArray())
                    .ToArray()
            )
        );
        foreach (var (file, contents) in FontSourceCatalog.Load(cache))
        {
            files.Add(($"fonts/{file}", contents));
        }
        return files.OrderBy(file => file.Name, StringComparer.Ordinal).ToList();
    }

    private GameAsset FindTerminalText()
    {
        GameAsset adapter = game.GetAssetsOfClass("TerminalListAdapter").First();
        GameAsset prefab = Require(
            game.Dereference(adapter, adapter.Fields["_Params"]["_ItemPrefab"]),
            "the item prefab"
        );
        GameAsset holder = Require(
            game.Dereference(prefab, prefab.Fields["m_GameObject"]),
            "the prefab's game object"
        );
        foreach (var entry in holder.Fields["m_Component.Array"].Children)
        {
            GameAsset? component = game.Dereference(holder, entry["component"]);
            if (component != null && game.GetClass(component) == "TextMeshProUGUI")
            {
                return game.GetAssetsOfClass("TextMeshProUGUI")
                    .First(candidate =>
                        candidate.File.name == component.File.name
                        && candidate.Info.PathId == component.Info.PathId
                    );
            }
        }
        throw new InvalidDataException("the terminal's line prefab has no TextMeshProUGUI");
    }

    private JsonObject ExtractFrame()
    {
        AssetTypeValueField parameters = game.GetAssetsOfClass("TerminalListAdapter")
            .First()
            .Fields["_Params"];
        AssetTypeValueField padding = parameters["_ContentPadding"];
        int[] sides = new[] { "m_Left", "m_Right", "m_Top", "m_Bottom" }
            .Select(side => padding[side].AsInt)
            .ToArray();
        if (sides.Distinct().Count() != 1)
        {
            throw new InvalidDataException(
                $"the terminal's content padding differs by side: {string.Join(", ", sides)}"
            );
        }
        return new JsonObject
        {
            ["padding"] = (float)sides[0],
            ["itemGap"] = ExtractNumber(parameters["_ContentSpacing"]),
            ["itemMinHeight"] = ExtractNumber(parameters["_DefaultItemSize"]),
        };
    }

    private JsonObject ExtractTheme()
    {
        string theme = Decode(
            WorldExtractor.ReadContents(game.GetResources("textfiles/defaulttheme").First())
        );
        JsonObject ExtractColor(string key)
        {
            var match = System.Text.RegularExpressions.Regex.Match(
                theme,
                $"<{key}>(.*?)</{key}>",
                System.Text.RegularExpressions.RegexOptions.Singleline
            );
            if (!match.Success)
            {
                throw new InvalidDataException($"the default theme has no {key}");
            }
            var color = new JsonObject();
            foreach (string channel in new[] { "r", "g", "b", "a" })
            {
                var value = System.Text.RegularExpressions.Regex.Match(
                    match.Groups[1].Value,
                    $"<{channel}>(\\d+)</{channel}>"
                );
                color[channel] = int.Parse(
                    value.Groups[1].Value,
                    System.Globalization.CultureInfo.InvariantCulture
                );
            }
            return color;
        }
        return new JsonObject
        {
            ["text"] = ExtractColor("terminalText"),
            ["background"] = ExtractColor("terminalBackground"),
        };
    }

    private JsonObject ExtractSprites(GameAsset asset)
    {
        AssetTypeValueField tree = asset.Fields;
        GameAsset texture = Require(
            game.Dereference(asset, tree["spriteSheet"]),
            "the sprite sheet"
        );
        var (width, height, bgra) = game.ReadPixels(texture);
        files.Add(($"sprites/{asset.Name}.rgba", ToRgba(bgra)));
        return new JsonObject
        {
            ["name"] = asset.Name,
            ["sheet"] = $"{asset.Name}.png",
            ["sheetWidth"] = width,
            ["sheetHeight"] = height,
            ["face"] = ExtractNumbers(tree["m_FaceInfo"]),
            ["glyphs"] = new JsonArray(
                tree["m_SpriteGlyphTable.Array"]
                    .Children.Select(glyph =>
                        (JsonNode?)
                            new JsonObject
                            {
                                ["index"] = ExtractNumber(glyph["m_Index"]),
                                ["x"] = ExtractNumber(glyph["m_GlyphRect"]["m_X"]),
                                ["y"] = ExtractNumber(glyph["m_GlyphRect"]["m_Y"]),
                                ["rectWidth"] = ExtractNumber(glyph["m_GlyphRect"]["m_Width"]),
                                ["rectHeight"] = ExtractNumber(glyph["m_GlyphRect"]["m_Height"]),
                                ["advance"] = ExtractNumber(
                                    glyph["m_Metrics"]["m_HorizontalAdvance"]
                                ),
                                ["bearingX"] = ExtractNumber(
                                    glyph["m_Metrics"]["m_HorizontalBearingX"]
                                ),
                                ["bearingY"] = ExtractNumber(
                                    glyph["m_Metrics"]["m_HorizontalBearingY"]
                                ),
                                ["width"] = ExtractNumber(glyph["m_Metrics"]["m_Width"]),
                                ["height"] = ExtractNumber(glyph["m_Metrics"]["m_Height"]),
                                ["scale"] = ExtractNumber(glyph["m_Scale"]),
                            }
                    )
                    .ToArray()
            ),
            ["characters"] = new JsonArray(
                tree["m_SpriteCharacterTable.Array"]
                    .Children.Select(character =>
                        (JsonNode?)
                            new JsonObject
                            {
                                ["name"] = character["m_Name"].AsString,
                                ["unicode"] = ExtractNumber(character["m_Unicode"]),
                                ["glyph"] = ExtractNumber(character["m_GlyphIndex"]),
                                ["scale"] = ExtractNumber(character["m_Scale"]),
                            }
                    )
                    .ToArray()
            ),
        };
    }

    private JsonObject ExtractTextAssets(GameAsset sprites)
    {
        var fonts = new SortedSet<string>(StringComparer.Ordinal);
        var materials = new SortedSet<string>(StringComparer.Ordinal);
        foreach (var (path, asset) in game.Resources)
        {
            if (asset == null || !path.StartsWith("fonts & materials/", StringComparison.Ordinal))
            {
                continue;
            }
            if (asset.Info.TypeId == (int)AssetClassID.Material)
            {
                string file = path[(path.LastIndexOf('/') + 1)..];
                string? owner = game.GetResources(path)
                    .Select(candidate => candidate.Name)
                    .FirstOrDefault(name => name.Equals(file, StringComparison.OrdinalIgnoreCase));
                materials.Add(owner ?? asset.Name);
            }
            else if (game.GetClass(asset) == "TMP_FontAsset")
            {
                fonts.Add(asset.Name);
            }
        }
        GameAsset settings = game.GetAssetsOfClass("TMP_Settings").First();
        GameAsset sheet = game.GetAssetsOfClass("TMP_StyleSheet")
            .First(candidate =>
                Require(
                    game.Dereference(settings, settings.Fields["m_defaultStyleSheet"]),
                    "the default style sheet"
                )
                    is var target
                && candidate.File.name == target.File.name
                && candidate.Info.PathId == target.Info.PathId
            );
        return new JsonObject
        {
            ["fonts"] = new JsonArray(fonts.Select(name => (JsonNode?)name).ToArray()),
            ["materials"] = new JsonArray(materials.Select(name => (JsonNode?)name).ToArray()),
            ["sprites"] = new JsonArray(
                sprites
                    .Fields["m_SpriteCharacterTable.Array"]
                    .Children.Select(character => (JsonNode?)character["m_Name"].AsString)
                    .ToArray()
            ),
            ["styles"] = new JsonArray(
                GetArray(sheet.Fields, "m_StyleList")
                    .Select(style =>
                        (JsonNode?)
                            new JsonObject
                            {
                                ["name"] = style["m_Name"].AsString,
                                ["openMarkup"] = style["m_OpeningDefinition"].AsString,
                                ["closeMarkup"] = style["m_ClosingDefinition"].AsString,
                            }
                    )
                    .ToArray()
            ),
        };
    }

    private static byte[] ToRgba(byte[] bgra)
    {
        byte[] rgba = new byte[bgra.Length];
        for (int pixel = 0; pixel < bgra.Length; pixel += 4)
        {
            rgba[pixel] = bgra[pixel + 2];
            rgba[pixel + 1] = bgra[pixel + 1];
            rgba[pixel + 2] = bgra[pixel];
            rgba[pixel + 3] = bgra[pixel + 3];
        }
        return rgba;
    }

    private static string Decode(byte[] data)
    {
        System.Text.Encoding encoding = data switch
        {
            [0xFF, 0xFE, ..] => System.Text.Encoding.Unicode,
            [0xFE, 0xFF, ..] => System.Text.Encoding.BigEndianUnicode,
            _ => System.Text.Encoding.UTF8,
        };
        return encoding.GetString(data).TrimStart('\uFEFF');
    }
}
