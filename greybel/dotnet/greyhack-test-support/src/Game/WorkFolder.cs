using GreyHackTestSupport.Platform;
using GreyHackTestSupport.Session;
using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Game;

/// <summary>
/// Contains a game's installed-game clone, downloads, plugin builds, game log, and shared-session files.
/// </summary>
internal sealed class WorkFolder(string folder)
{
    public string Folder { get; } = folder;

    public string Clone => GetPath("game");

    public string Downloads => GetPath("downloads");

    public string PluginBuilds => GetPath("plugins");

    public string PlayerLog => GetPath("player.log");

    /// <summary>Session file from which <see cref="GameSession"/> reads the active host and ports.</summary>
    public string SessionFile => GetPath("headless.json");

    /// <summary>Stop file used to request shared-session host termination.</summary>
    public string StopFile => GetPath("headless.stop");

    /// <summary>Uses the specified folder or defaults to <c>target/greyhack-game-&lt;major&gt;</c>.</summary>
    public static WorkFolder FromOption(DirectoryInfo? folder, BepInExMajor major) =>
        new(folder?.FullName ?? Path.Combine(Repository.Target, $"greyhack-game-{(int)major}"));

    /// <summary>Acquires the game lock, limiting each work folder to one running game across all launchers.</summary>
    public WorkFolderLock LockGame() => WorkFolderLock.Take(Folder);

    /// <summary>Acquires the session lock used to serialize shared-session startup and shutdown.</summary>
    public WorkFolderLock LockSession() => WorkFolderLock.Take(GetPath("session"));

    private string GetPath(string name) => Path.Combine(Folder, name);
}
