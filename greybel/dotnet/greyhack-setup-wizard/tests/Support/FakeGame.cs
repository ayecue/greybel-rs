using System.IO.Compression;
using GreyHackSetupWizard.Game;

namespace GreyHackSetupWizard.Tests.Support;

internal static class FakeGame
{
    public static GameLayout Create(TemporaryFolder folder)
    {
        Directory.CreateDirectory(
            System.IO.Path.Combine(
                folder.Path,
                "Grey Hack.app",
                "Contents",
                "Resources",
                "Data",
                "Managed"
            )
        );
        return GameLayout.FromFolder(folder.Path);
    }

    public static string CreateArchive(TemporaryFolder folder, string name, params string[] files)
    {
        string path = folder.GetPath(name);
        using ZipArchive zip = ZipFile.Open(path, ZipArchiveMode.Create);
        foreach (string file in files)
        {
            using var writer = new StreamWriter(zip.CreateEntry(file).Open());
            writer.Write(file);
        }
        return path;
    }

    public static string CreateArchive(
        TemporaryFolder folder,
        string name,
        IReadOnlyDictionary<string, string> files
    )
    {
        string path = folder.GetPath(name);
        using ZipArchive zip = ZipFile.Open(path, ZipArchiveMode.Create);
        foreach (var (file, content) in files)
        {
            using var writer = new StreamWriter(zip.CreateEntry(file).Open());
            writer.Write(content);
        }
        return path;
    }

    public static void Write(GameLayout game, string path, string text = "")
    {
        string full = System.IO.Path.Combine(game.Folder, path);
        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(full)!);
        System.IO.File.WriteAllText(full, text);
    }
}
