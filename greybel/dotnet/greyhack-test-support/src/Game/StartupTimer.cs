using System.Diagnostics;

namespace GreyHackTestSupport.Game;

/// <summary>Writes each game startup step duration and total startup duration to standard error.</summary>
internal sealed class StartupTimer
{
    private readonly Stopwatch total = Stopwatch.StartNew();
    private readonly Stopwatch step = Stopwatch.StartNew();

    /// <summary>Records the interval since the previous step under <paramref name="name"/>.</summary>
    public void Finish(string name)
    {
        Report(name, step);
        step.Restart();
    }

    public void FinishTotal() => Report("total", total);

    private static void Report(string name, Stopwatch watch) =>
        Console.Error.WriteLine($"[greyhack-test-support] {name}: {watch.ElapsedMilliseconds} ms");
}
