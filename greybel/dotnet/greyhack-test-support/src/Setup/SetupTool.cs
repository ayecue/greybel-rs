using System.Text.Json;
using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Setup;

/// <summary>
/// Sets up the game copy with greyhack-setup, the command line of <c>apps/rs/greyhack-setup-wizard</c> built without
/// its window: it finds the installed game, installs BepInEx and the message hook, and sets the hook's port.
/// </summary>
internal static class SetupTool
{
    /// <summary>The installed game: <c>GREY_HACK_PATH</c>, else the one in the Steam libraries.</summary>
    public static GameLayout Locate() => ReadGame(Run(["status"]));

    public static GameLayout Read(string folder) => ReadGame(Run(["status", "--game", folder]));

    /// <summary>
    /// Installs greyhack-setup's BepInEx release of <paramref name="major"/> unless the game has it, downloading into
    /// <paramref name="downloads"/>.
    /// </summary>
    public static void EnsureBepInEx(GameLayout game, BepInExMajor major, string downloads)
    {
        JsonElement status = Run(["status", "--game", game.Folder]);
        JsonElement release = status
            .GetProperty("bepinexReleases")
            .EnumerateArray()
            .Single(release => release.GetProperty("major").GetInt32() == (int)major);
        JsonElement installation = status.GetProperty("status").GetProperty("bepinex");
        if (
            installation.ValueKind == JsonValueKind.Object
            && installation.GetProperty("major").GetInt32() == (int)major
            && installation.GetProperty("version").GetString()
                == release.GetProperty("version").GetString()
        )
        {
            return;
        }
        Run([
            "bepinex",
            "install",
            "--bepinex",
            ((int)major).ToString(),
            "--game",
            game.Folder,
            "--cache",
            downloads,
            "--yes",
        ]);
    }

    /// <summary>Installs the message hook <paramref name="assembly"/>, the build for <paramref name="major"/>.</summary>
    public static void InstallHook(GameLayout game, BepInExMajor major, string assembly) =>
        Run([
            "hook",
            "install",
            "--assembly",
            assembly,
            "--bepinex",
            ((int)major).ToString(),
            "--game",
            game.Folder,
        ]);

    public static void SetPort(GameLayout game, int port) =>
        Run(["configure", "--port", port.ToString(), "--game", game.Folder]);

    /// <summary>Runs greyhack-setup with <c>--json</c>; returns its last event, or throws its error.</summary>
    private static JsonElement Run(IEnumerable<string> arguments)
    {
        ExternalProcess run = ExternalProcess.Run(
            "cargo",
            [
                "run",
                "--quiet",
                "--manifest-path",
                Repository.AppsManifest,
                "--package",
                "greyhack-setup-wizard",
                "--no-default-features",
                "--",
                .. arguments,
                "--json",
            ]
        );
        JsonElement? last = null;
        foreach (string line in run.Output.Split('\n'))
        {
            if (!line.StartsWith('{'))
            {
                continue;
            }
            JsonElement element = JsonDocument.Parse(line).RootElement.Clone();
            if (element.GetProperty("type").GetString() == "error")
            {
                throw new TestSupportException(
                    $"greyhack-setup: {element.GetProperty("message").GetString()}"
                );
            }
            last = element;
        }
        return run.Succeeded && last is { } result
            ? result
            : throw new TestSupportException($"greyhack-setup failed:\n{run.Output}");
    }

    private static GameLayout ReadGame(JsonElement status)
    {
        JsonElement game = status.GetProperty("status");
        return new GameLayout(
            game.GetProperty("folder").GetString()!,
            game.GetProperty("executable").GetString()!
        );
    }
}
