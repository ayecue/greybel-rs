namespace GreyHackUtil.Assets;

internal static class WorldExtractor
{
    private static readonly Dictionary<string, string> ConfigFiles = new()
    {
        ["BaseFileSystem"] = "base_file_system.json",
        ["Servicios"] = "services.json",
        ["TipoRedPublica"] = "public_services.json",
        ["Encode"] = "encode.src",
        ["Decode"] = "decode.bin",
        ["ReservedIPs"] = "reserved_ips.json",
        ["WorldItems"] = "world_items.json",
        ["MissionProcedural"] = "mission_procedural.json",
        ["MissionHidden"] = "mission_hidden.json",
        ["MissionKarma"] = "mission_karma.json",
        ["MissionKarmaBlack"] = "mission_karma_black.json",
        ["MissionKarmaWhite"] = "mission_karma_white.json",
        ["MissionZeroDay"] = "mission_zero_day.json",
    };

    private static readonly Dictionary<string, string> NameFiles = new()
    {
        ["CorpNames"] = "corporations",
        ["exploitNames"] = "exploits",
        ["female-names"] = "female_names",
        ["male-names"] = "male_names",
        ["lib_variables"] = "library_variables",
        ["Passwords"] = "passwords",
        ["Surnamesmod"] = "surnames",
        ["usernames"] = "usernames",
    };

    public static List<(string Name, byte[] Contents)> Extract(GameData game)
    {
        var files = new List<(string Name, byte[] Contents)>();
        foreach (var (path, asset) in game.Resources)
        {
            if (asset != null && path.StartsWith("textfiles/names/", StringComparison.Ordinal))
            {
                string file = $"names/{NameFiles[asset.Name]}.txt";
                files.Add((file, NameShuffle.Shuffle(file, ReadContents(asset))));
            }
        }
        GameAsset texts = game.GetAssetsOfClass("GlobalText").First();
        foreach (var pointer in texts.Fields["commands.Array"].Children)
        {
            GameAsset script = Require(game.Dereference(texts, pointer), "a command");
            files.Add(($"scripts/{script.Name}.src", ReadContents(script)));
        }
        foreach (var pointer in texts.Fields["exploitsParts.Array"].Children)
        {
            GameAsset part = Require(game.Dereference(texts, pointer), "an exploit part");
            files.Add(($"exploits/{part.Name.ToLowerInvariant()}.src", ReadContents(part)));
        }
        foreach (var pointer in texts.Fields["configGeneration.Array"].Children)
        {
            GameAsset config = Require(game.Dereference(texts, pointer), "a configuration file");
            if (config.Name == "GameTexts_en")
            {
                files.Add(("texts/en.json", ReadContents(config)));
            }
            else if (ConfigFiles.TryGetValue(config.Name, out string? file))
            {
                files.Add(($"config/{file}", ReadContents(config)));
            }
        }
        foreach (var pointer in texts.Fields["textFiles.Array"].Children)
        {
            GameAsset text = Require(game.Dereference(texts, pointer), "a text file");
            files.Add(($"texts/files/{text.Name}.txt", ReadContents(text)));
        }
        GameAsset website = Require(
            game.Dereference(texts, texts.Fields["defaultTextFiles"]),
            "the default website"
        );
        files.Add(("config/default.html", ReadContents(website)));
        return files.OrderBy(file => file.Name, StringComparer.Ordinal).ToList();
    }

    public static byte[] ReadContents(GameAsset asset) => asset.Fields["m_Script"].AsByteArray;

    private static GameAsset Require(GameAsset? asset, string what) =>
        asset ?? throw new InvalidDataException($"GlobalText points to no {what}");
}
