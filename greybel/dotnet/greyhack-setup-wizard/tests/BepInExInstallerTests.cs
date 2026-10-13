using System.Runtime.Versioning;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class BepInExInstallerTests
{
    [Fact]
    public void ShouldReplaceTheLoaderAndCoreButNotThePluginsAndConfig()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        FakeGame.Write(game, "BepInEx/core/BepInEx.dll");
        FakeGame.Write(game, "BepInEx/core/Old.dll");
        FakeGame.Write(game, "libdoorstop.dylib", "old");
        FakeGame.Write(game, "BepInEx/plugins/Mine.dll");
        FakeGame.Write(game, "BepInEx/config/Mine.cfg");
        string archive = FakeGame.CreateArchive(
            folder,
            "beta.zip",
            "run_bepinex.sh",
            "libdoorstop.dylib",
            "BepInEx/core/BepInEx.Unity.Mono.dll",
            "BepInEx/core/BepInEx.Core.dll"
        );

        BepInExInstallation installed = BepInExInstaller.Install(game, archive);

        Assert.Equal(BepInExMajor.Beta, installed.Major);
        Assert.False(File.Exists(Path.Combine(game.BepInExFolder, "core", "Old.dll")));
        Assert.Equal(
            "libdoorstop.dylib",
            File.ReadAllText(Path.Combine(game.Folder, "libdoorstop.dylib"))
        );
        Assert.True(File.Exists(Path.Combine(game.PluginsFolder, "Mine.dll")));
        Assert.True(File.Exists(Path.Combine(game.ConfigFolder, "Mine.cfg")));
    }

    [Fact]
    [SupportedOSPlatform("macos")]
    [SupportedOSPlatform("linux")]
    public void ShouldMakeTheLauncherExecutable()
    {
        Assert.SkipWhen(OperatingSystem.IsWindows(), "Windows has no file modes");

        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string archive = FakeGame.CreateArchive(
            folder,
            "stable.zip",
            "run_bepinex.sh",
            "BepInEx/core/BepInEx.dll"
        );

        BepInExInstaller.Install(game, archive);

        Assert.True(
            File.GetUnixFileMode(Path.Combine(game.Folder, "run_bepinex.sh"))
                .HasFlag(UnixFileMode.UserExecute)
        );
    }

    [Fact]
    public void ShouldRunTheBetaAsX64OnMacOS()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string archive = FakeGame.CreateArchive(
            folder,
            "beta.zip",
            new Dictionary<string, string>
            {
                ["run_bepinex.sh"] = "#!/bin/sh\narchpreference=\"arm64,x86_64\"\n",
                ["BepInEx/core/BepInEx.Unity.Mono.dll"] = "",
            }
        );

        BepInExInstaller.Install(game, archive);

        Assert.Equal("x86_64", BepInExLauncher.GetArchitectures(game));
    }

    [Fact]
    public void ShouldRunTheStableAsTheLauncherPrefers()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string archive = FakeGame.CreateArchive(
            folder,
            "stable.zip",
            new Dictionary<string, string>
            {
                ["run_bepinex.sh"] = "#!/bin/sh\narchpreference=\"arm64,x86_64\"\n",
                ["BepInEx/core/BepInEx.dll"] = "",
            }
        );

        BepInExInstaller.Install(game, archive);

        Assert.Equal("arm64,x86_64", BepInExLauncher.GetArchitectures(game));
    }

    [Fact]
    public void ShouldRejectAnArchiveWithoutBepInExsCore()
    {
        using var folder = new TemporaryFolder();
        GameLayout game = FakeGame.Create(folder);
        string archive = FakeGame.CreateArchive(folder, "other.zip", "readme.txt");

        var error = Assert.Throws<SetupException>(() => BepInExInstaller.Install(game, archive));

        Assert.EndsWith("isn't a BepInEx zip file (it has no BepInEx/core folder).", error.Message);
    }

    [Fact]
    public void ShouldDetectNoBepInExInAGameWithout()
    {
        using var folder = new TemporaryFolder();

        BepInExInstallation? installation = BepInExInstallation.Detect(FakeGame.Create(folder));

        Assert.Null(installation);
    }
}
