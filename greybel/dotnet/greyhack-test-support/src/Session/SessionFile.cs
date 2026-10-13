using System.Diagnostics;
using GreyHackTestSupport.Game;
using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Session;

/// <summary>
/// Controls a work folder's shared session through two files. The host writes the session file when the game is
/// ready, and each use updates its timestamp for idle detection. The stop file requests host termination.
/// </summary>
internal sealed class SessionFile(WorkFolder folder)
{
    public void Write(SessionRecord record) =>
        File.WriteAllText(folder.SessionFile, record.ToJson());

    /// <summary>Gets the session record only when its host is running and both ports accept connections.</summary>
    public SessionRecord? ReadLive()
    {
        SessionRecord? record = Read();
        if (record is null || !IsRunning(record.Pid))
        {
            return null;
        }
        return LocalPort.Listens(record.Port) && LocalPort.Listens(record.TestPort) ? record : null;
    }

    /// <summary>Gets the session record without checking host status.</summary>
    public SessionRecord? Read() =>
        File.Exists(folder.SessionFile)
            ? SessionRecord.Parse(File.ReadAllText(folder.SessionFile))
            : null;

    /// <summary>Updates the session's last-used timestamp.</summary>
    public void UpdateLastUse() => File.SetLastWriteTimeUtc(folder.SessionFile, DateTime.UtcNow);

    /// <summary>True when no session use occurred for longer than <paramref name="idleTimeout"/>.</summary>
    public bool IsIdle(TimeSpan idleTimeout) =>
        DateTime.UtcNow - File.GetLastWriteTimeUtc(folder.SessionFile) > idleTimeout;

    public void RequestStop() => File.WriteAllText(folder.StopFile, "");

    public bool IsStopRequested => File.Exists(folder.StopFile);

    /// <summary>Removes the session and stop files.</summary>
    public void Clear()
    {
        File.Delete(folder.SessionFile);
        File.Delete(folder.StopFile);
    }

    private static bool IsRunning(int pid)
    {
        try
        {
            using Process host = Process.GetProcessById(pid);
            return !host.HasExited;
        }
        catch (ArgumentException)
        {
            return false;
        }
    }
}
