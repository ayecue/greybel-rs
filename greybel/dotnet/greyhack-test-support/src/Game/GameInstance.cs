using System.Runtime.InteropServices;
using GreyHackTestSupport.Platform;
using GreyHackTestSupport.Session;
using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Game;

/// <summary>
/// Represents the installed game running headless in the work-folder clone with the message hook and test support
/// plugin. Initialization completes after the player desktop boots and both listeners accept connections. Disposal,
/// SIGTERM, and SIGINT kill the game and release the work folder.
/// </summary>
internal sealed class GameInstance : IDisposable
{
    private static readonly TimeSpan SteamStartTimeout = TimeSpan.FromSeconds(60);
    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan SessionPollInterval = TimeSpan.FromMilliseconds(250);

    private readonly WorkFolder folder;
    private readonly WorkFolderLock folderLock;
    private readonly List<PosixSignalRegistration> signals = [];
    private HeadlessGame? headless;

    private GameInstance(WorkFolder folder)
    {
        this.folder = folder;
        folderLock = folder.LockGame();
    }

    public GameConnection Connection { get; private set; } = new(0, 0, "");

    /// <summary>Prepares and starts the game in the work folder; throws if startup exceeds the time limit.</summary>
    public static GameInstance Start(GameOptions options)
    {
        var game = new GameInstance(options.WorkFolder);
        try
        {
            game.StartGame(options);
            return game;
        }
        catch
        {
            game.Dispose();
            throw;
        }
    }

    /// <summary>Waits for end of standard input and throws if the game exits first.</summary>
    public int RunUntilInputEnds()
    {
        var input = new Thread(() => Console.In.ReadToEnd()) { IsBackground = true };
        input.Start();
        while (input.IsAlive)
        {
            ThrowOnExit("while it ran");
            input.Join(PollInterval);
        }
        return 0;
    }

    /// <summary>
    /// Hosts the work folder's shared game session until the game exits, a stop is requested, or no use occurs for
    /// <paramref name="idleTimeout"/>.
    /// </summary>
    public int RunAsSession(TimeSpan idleTimeout)
    {
        var session = new SessionFile(folder);
        session.Write(SessionRecord.Create(Environment.ProcessId, Connection));
        while (!Headless.HasExited && !session.IsStopRequested && !session.IsIdle(idleTimeout))
        {
            Thread.Sleep(SessionPollInterval);
        }
        Headless.Dispose();
        session.Clear();
        return 0;
    }

    public void Dispose()
    {
        foreach (PosixSignalRegistration signal in signals)
        {
            signal.Dispose();
        }
        headless?.Dispose();
        folderLock.Dispose();
    }

    private HeadlessGame Headless =>
        headless ?? throw new InvalidOperationException("the game hasn't started");

    private void StartGame(GameOptions options)
    {
        var timer = new StartupTimer();
        GameClone.Ensure(SetupTool.Locate().Folder, folder.Clone);
        timer.Finish("game-clone");
        GameLayout game = SetupTool.Read(folder.Clone);
        BepInExSetup.Ensure(folder, game, options.Major);
        timer.Finish("bepinex-setup");
        PluginBuildTable.Install(folder, game, options.Major);
        timer.Finish("plugin-install");
        int port = LocalPort.Resolve(options.Port);
        int testPort = LocalPort.Resolve(options.TestPort);
        SetupTool.SetPort(game, port);
        TestSupportConfig.SetPorts(game, testPort, LocalPort.Resolve(0));
        Steam.Start(SteamStartTimeout);
        timer.Finish("steam-start");
        Connection = new GameConnection(port, testPort, folder.PlayerLog);
        File.Delete(Connection.Log);
        headless = HeadlessGame.Start(game, Connection.Log);
        foreach (PosixSignal signal in new[] { PosixSignal.SIGTERM, PosixSignal.SIGINT })
        {
            signals.Add(PosixSignalRegistration.Create(signal, _ => headless.Dispose()));
        }
        WaitUntilReady(options.Timeout);
        timer.Finish("desktop-ready");
        timer.FinishTotal();
    }

    private void WaitUntilReady(TimeSpan timeout)
    {
        var deadline = DateTime.UtcNow + timeout;
        while (DateTime.UtcNow < deadline)
        {
            ThrowOnExit("before loading single player (is Steam logged in?)");
            if (
                GameLog.Contains(Connection.Log, GameLog.DesktopReady)
                && LocalPort.Listens(Connection.Port)
                && LocalPort.Listens(Connection.TestPort)
            )
            {
                return;
            }
            Thread.Sleep(PollInterval);
        }
        throw GameLog.CreateFailure(
            $"the game didn't load single player and boot within {timeout.TotalSeconds} s",
            Connection.Log
        );
    }

    private void ThrowOnExit(string when)
    {
        if (Headless.HasExited)
        {
            throw GameLog.CreateFailure(
                $"the game exited with {Headless.ExitCode} {when}",
                Connection.Log
            );
        }
    }
}
