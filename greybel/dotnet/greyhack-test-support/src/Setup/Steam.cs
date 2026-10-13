using GreyHackTestSupport.Platform;

namespace GreyHackTestSupport.Setup;

/// <summary>Manages the running, authenticated Steam client required by the game.</summary>
internal static class Steam
{
    /// <summary>Grey Hack application ID used to prevent Steam from restarting the game.</summary>
    public const string AppId = "605230";

    private static readonly TimeSpan PollInterval = TimeSpan.FromMilliseconds(500);

    private static string ProcessName => OperatingSystem.IsMacOS() ? "steam_osx" : "steam";

    public static bool IsRunning() => ExternalProcess.Run("pgrep", "-x", ProcessName).Succeeded;

    /// <summary>Launches Steam without a window if necessary and waits for it to run.</summary>
    public static void Start(TimeSpan timeout)
    {
        if (IsRunning())
        {
            return;
        }
        ExternalProcess start = OperatingSystem.IsMacOS()
            ? ExternalProcess.Run("open", "-g", "-j", "-a", "Steam", "--args", "-silent")
            : ExternalProcess.Run("sh", "-c", "nohup steam -silent >/dev/null 2>&1 &");
        if (!start.Succeeded)
        {
            throw new TestSupportException(
                $"starting Steam failed with {start.ExitCode}: {start.Output}"
            );
        }
        var deadline = DateTime.UtcNow + timeout;
        while (!IsRunning())
        {
            if (DateTime.UtcNow > deadline)
            {
                throw new TestSupportException(
                    $"Steam didn't start within {timeout.TotalSeconds} s"
                );
            }
            Thread.Sleep(PollInterval);
        }
    }
}
