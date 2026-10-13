using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class SteamLibrariesTests
{
    [Fact]
    public void ShouldReadTheLibraryFoldersSteamLists()
    {
        string listing = File.ReadAllText(
            Path.Combine(AppContext.BaseDirectory, "Fixtures", "libraryfolders.vdf")
        );

        IReadOnlyList<string> libraries = SteamLibraryCatalog.Parse(listing);

        Assert.Equal([@"C:\Program Files (x86)\Steam", @"D:\SteamLibrary"], libraries);
    }

    [Fact]
    public void ShouldFindTheGameInALibraryWithItsManifest()
    {
        using var steam = new TemporaryFolder();
        using var library = new TemporaryFolder();
        Directory.CreateDirectory(Path.Combine(steam.Path, "steamapps"));
        File.WriteAllText(
            Path.Combine(steam.Path, "steamapps", "libraryfolders.vdf"),
            $"\"libraryfolders\" {{ \"1\" {{ \"path\" \"{library.Path}\" }} }}"
        );
        string game = Path.Combine(library.Path, "steamapps", "common", "Grey Hack");
        Directory.CreateDirectory(game);
        File.WriteAllText(
            Path.Combine(library.Path, "steamapps", $"appmanifest_{SteamLibraryCatalog.AppId}.acf"),
            ""
        );

        string? found = SteamLibraryCatalog.FindGame([steam.Path]);

        Assert.Equal(game, found);
    }

    [Fact]
    public void ShouldFindNoGameWithoutItsManifest()
    {
        using var steam = new TemporaryFolder();
        Directory.CreateDirectory(Path.Combine(steam.Path, "steamapps", "common", "Grey Hack"));

        string? found = SteamLibraryCatalog.FindGame([steam.Path]);

        Assert.Null(found);
    }
}
