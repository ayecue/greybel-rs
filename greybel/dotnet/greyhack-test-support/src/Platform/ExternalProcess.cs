using System.Diagnostics;

namespace GreyHackTestSupport.Platform;

/// <summary>Executes a program to completion while capturing standard output and standard error.</summary>
internal sealed record ExternalProcess(int ExitCode, string Output)
{
    public bool Succeeded => ExitCode == 0;

    public static ExternalProcess Run(string program, params IEnumerable<string> arguments)
    {
        var start = new ProcessStartInfo(program)
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };
        foreach (string argument in arguments)
        {
            start.ArgumentList.Add(argument);
        }
        using Process process =
            Process.Start(start) ?? throw new TestSupportException($"{program} doesn't start");
        Task<string> error = process.StandardError.ReadToEndAsync();
        string output = process.StandardOutput.ReadToEnd();
        process.WaitForExit();
        return new ExternalProcess(process.ExitCode, output + error.GetAwaiter().GetResult());
    }
}
