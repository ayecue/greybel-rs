using System.CommandLine;
using AssetBundle;
using GreyHackUtil.Assets;
using GreyHackUtil.Extraction;

namespace GreyHackUtil.Cli;

internal static class AssetsCommand
{
    public const string BundleName = "assets.bundle";

    public static Command Create()
    {
        Argument<DirectoryInfo> data = new Argument<DirectoryInfo>("data")
        {
            Description =
                "The game's data folder (Grey Hack_Data, or Contents/Resources/Data on macOS)",
        }.AcceptExistingOnly();
        Option<DirectoryInfo> world = new("--world")
        {
            Description =
                "Folder to write greyhack-world-assets' assets.bundle and fingerprint.json into",
            Required = true,
        };
        Option<DirectoryInfo> textMesh = new("--text-mesh")
        {
            Description =
                "Folder to write greyhack-text-mesh-assets' assets.bundle and fingerprint.json into",
            Required = true,
        };
        Option<DirectoryInfo> cache = new("--cache")
        {
            Description = "Folder to keep downloads in: the type tree dump and the font files",
            Required = true,
        };

        var command = new Command(
            "assets",
            "Read the game's world data and terminal text assets into bundles"
        )
        {
            data,
            world,
            textMesh,
            cache,
        };
        command.SetAction(result =>
            CommandAction.Run(() =>
            {
                string folder = result.GetRequiredValue(data).FullName;
                string version =
                    new GameAssembly(
                        Path.Combine(folder, "Managed", "Assembly-CSharp.dll")
                    ).GameVersion
                    ?? throw new InvalidDataException(
                        "the game's assembly has no GameConfig.gameVersion"
                    );
                var game = new GameData(folder, result.GetRequiredValue(cache).FullName);
                Fingerprint.WriteBundle(
                    result.GetRequiredValue(world).FullName,
                    BundleName,
                    version,
                    WorldExtractor.Extract(game)
                );
                Fingerprint.WriteBundle(
                    result.GetRequiredValue(textMesh).FullName,
                    BundleName,
                    version,
                    new TextMeshExtractor(game).Extract(result.GetRequiredValue(cache).FullName)
                );
            })
        );
        return command;
    }
}
