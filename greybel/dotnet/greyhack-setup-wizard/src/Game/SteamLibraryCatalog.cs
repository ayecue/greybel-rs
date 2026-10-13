using System.Text.RegularExpressions;

namespace GreyHackSetupWizard.Game;

public static partial class SteamLibraryCatalog
{
    public const string AppId = "605230";

    public static IEnumerable<string> GetSteamFolders(Platform platform)
    {
        string home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        return platform switch
        {
            Platform.Windows =>
            [
                Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.ProgramFilesX86),
                    "Steam"
                ),
                Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
                    "Steam"
                ),
            ],
            Platform.MacOS => [Path.Combine(home, "Library", "Application Support", "Steam")],
            _ =>
            [
                Path.Combine(home, ".steam", "steam"),
                Path.Combine(home, ".local", "share", "Steam"),
                Path.Combine(
                    home,
                    ".var",
                    "app",
                    "com.valvesoftware.Steam",
                    ".local",
                    "share",
                    "Steam"
                ),
            ],
        };
    }

    public static IReadOnlyList<string> Parse(string libraryFolders) =>
        GetPathEntryRegex()
            .Matches(libraryFolders)
            .Select(match => match.Groups[1].Value.Replace(@"\\", @"\"))
            .ToList();

    public static string? FindGame(Platform platform) => FindGame(GetSteamFolders(platform));

    public static string? FindGame(IEnumerable<string> steamFolders)
    {
        foreach (string steam in steamFolders)
        {
            string libraryFile = Path.Combine(steam, "steamapps", "libraryfolders.vdf");
            IEnumerable<string> libraries = File.Exists(libraryFile)
                ? Parse(File.ReadAllText(libraryFile)).Prepend(steam)
                : [steam];
            foreach (string library in libraries.Distinct())
            {
                string apps = Path.Combine(library, "steamapps");
                string game = Path.Combine(apps, "common", "Grey Hack");
                if (
                    File.Exists(Path.Combine(apps, $"appmanifest_{AppId}.acf"))
                    && Directory.Exists(game)
                )
                {
                    return game;
                }
            }
        }
        return null;
    }

    [GeneratedRegex("\"path\"\\s+\"([^\"]*)\"")]
    private static partial Regex GetPathEntryRegex();
}
