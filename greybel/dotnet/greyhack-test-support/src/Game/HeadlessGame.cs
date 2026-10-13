using System.Diagnostics;
using GreyHackSetupWizard.Game;
using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Game;

internal sealed class HeadlessGame : IDisposable
{
    private readonly Process process;

    private HeadlessGame(Process process)
    {
        this.process = process;
    }

    public static HeadlessGame Start(GameLayout game, string log)
    {
        var start = new ProcessStartInfo(Path.Combine(game.Folder, "run_bepinex.sh"))
        {
            WorkingDirectory = game.Folder,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        foreach (
            string argument in new[]
            {
                game.Executable,
                "-batchmode",
                "-nographics",
                "-logFile",
                log,
            }
        )
        {
            start.ArgumentList.Add(argument);
        }
        start.Environment["SteamAppId"] = Steam.AppId;
        Process process =
            Process.Start(start) ?? throw new TestSupportException("the game doesn't start");
        process.OutputDataReceived += (_, _) => { };
        process.ErrorDataReceived += (_, _) => { };
        process.BeginOutputReadLine();
        process.BeginErrorReadLine();
        return new HeadlessGame(process);
    }

    public bool HasExited => process.HasExited;

    public int ExitCode => process.ExitCode;

    public void Dispose()
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
                process.WaitForExit();
            }
        }
        catch (InvalidOperationException) { }
        process.Dispose();
    }
}
