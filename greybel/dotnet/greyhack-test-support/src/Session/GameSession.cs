using System.Diagnostics;
using System.Globalization;
using System.Reflection;
using GreyHackTestSupport.Game;
using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Session;

/// <summary>
/// Manages one detached game per work folder. The first <c>game start</c> launches it; subsequent calls from any
/// test process reuse it until <c>game stop</c> or the idle timeout expires.
/// </summary>
internal static class GameSession
{
    private static readonly TimeSpan StopTimeout = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan HostStartGrace = TimeSpan.FromSeconds(30);
    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(100);

    /// <summary>Starts the host if necessary and prints the active session record.</summary>
    public static int Start(GameOptions options, TimeSpan idleTimeout, TextWriter output)
    {
        using WorkFolderLock sessionLock = options.WorkFolder.LockSession();
        var session = new SessionFile(options.WorkFolder);
        SessionRecord? record = session.ReadLive();
        if (record is null)
        {
            session.Clear();
            record = StartHost(options, idleTimeout, session);
        }
        session.UpdateLastUse();
        output.WriteLine(record.ToJson());
        return 0;
    }

    /// <summary>Sends a host stop request and waits for termination.</summary>
    public static int Stop(WorkFolder folder)
    {
        using WorkFolderLock sessionLock = folder.LockSession();
        var session = new SessionFile(folder);
        session.RequestStop();
        if (session.Read() is { } record)
        {
            try
            {
                using Process host = Process.GetProcessById(record.Pid);
                host.WaitForExit(StopTimeout);
            }
            catch (ArgumentException) { }
        }
        session.Clear();
        return 0;
    }

    /// <summary>Launches a detached <c>game host</c> and waits for its session to become available.</summary>
    private static SessionRecord StartHost(
        GameOptions options,
        TimeSpan idleTimeout,
        SessionFile session
    )
    {
        using Process host = StartHostProcess(options, idleTimeout);
        var deadline = DateTime.UtcNow + options.Timeout + HostStartGrace;
        while (DateTime.UtcNow < deadline)
        {
            if (session.ReadLive() is { } record)
            {
                return record;
            }
            if (host.HasExited)
            {
                throw new TestSupportException($"the game host exited with {host.ExitCode}");
            }
            Thread.Sleep(PollInterval);
        }
        session.RequestStop();
        throw new TestSupportException("the game session didn't become ready");
    }

    private static Process StartHostProcess(GameOptions options, TimeSpan idleTimeout)
    {
        string executable = Environment.ProcessPath ?? "dotnet";
        var start = new ProcessStartInfo(executable)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
        };
        if (
            Path.GetFileNameWithoutExtension(executable)
                .Equals("dotnet", StringComparison.OrdinalIgnoreCase)
        )
        {
            start.ArgumentList.Add("exec");
            start.ArgumentList.Add(
                Assembly.GetEntryAssembly()?.Location
                    ?? throw new TestSupportException("the tool has no entry assembly")
            );
        }
        foreach (string argument in GetHostArguments(options, idleTimeout))
        {
            start.ArgumentList.Add(argument);
        }
        Process host =
            Process.Start(start) ?? throw new TestSupportException("the game host doesn't start");
        host.BeginOutputReadLine();
        return host;
    }

    private static IEnumerable<string> GetHostArguments(
        GameOptions options,
        TimeSpan idleTimeout
    ) =>
        [
            "game",
            "host",
            "--work-folder",
            options.WorkFolder.Folder,
            "--bepinex",
            ((int)options.Major).ToString(CultureInfo.InvariantCulture),
            "--port",
            options.Port.ToString(CultureInfo.InvariantCulture),
            "--test-port",
            options.TestPort.ToString(CultureInfo.InvariantCulture),
            "--timeout",
            options.Timeout.TotalSeconds.ToString(CultureInfo.InvariantCulture),
            "--idle-timeout",
            idleTimeout.TotalSeconds.ToString(CultureInfo.InvariantCulture),
        ];
}
