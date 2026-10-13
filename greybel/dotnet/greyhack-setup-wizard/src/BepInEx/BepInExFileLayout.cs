using GreyHackSetupWizard.Game;

namespace GreyHackSetupWizard.BepInEx;

public static class BepInExFileLayout
{
    public static readonly IReadOnlyList<string> LoaderFiles =
    [
        "run_bepinex.sh",
        "libdoorstop.dylib",
        "libdoorstop.so",
        "libdoorstop_x64.so",
        "libdoorstop_x86.so",
        "winhttp.dll",
        "doorstop_config.ini",
        ".doorstop_version",
        "changelog.txt",
    ];

    public static readonly IReadOnlyList<string> Folders =
    [
        "core",
        "patchers",
        "cache",
        "unity-libs",
        "interop",
    ];

    public static void Remove(GameLayout game)
    {
        foreach (string file in LoaderFiles)
        {
            File.Delete(Path.Combine(game.Folder, file));
        }
        foreach (string folder in Folders)
        {
            string path = Path.Combine(game.BepInExFolder, folder);
            if (Directory.Exists(path))
            {
                Directory.Delete(path, recursive: true);
            }
        }
        foreach (
            string log in Directory.Exists(game.BepInExFolder)
                ? Directory.EnumerateFiles(game.BepInExFolder, "LogOutput*.log")
                : []
        )
        {
            File.Delete(log);
        }
    }
}
