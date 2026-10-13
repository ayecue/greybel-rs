namespace GreyHackSetupWizard.Cli.Commands;

internal sealed class ConsoleProgress : IProgress<string>
{
    public static readonly ConsoleProgress Instance = new();

    public void Report(string value) => Console.WriteLine(value);
}
