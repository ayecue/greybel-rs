using GreyHackSetupWizard.Cli.Resources;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class CommandRunner
{
    public static int Run(Func<Task<int>> action)
    {
        try
        {
            return action().GetAwaiter().GetResult();
        }
        catch (Exception error)
            when (error
                    is SetupException
                        or IOException
                        or UnauthorizedAccessException
                        or InvalidDataException
            )
        {
            Console.Error.WriteLine(Localization.Format(CommandText.ErrorLine, error.Message));
            return 1;
        }
    }

    public static int Run(Action action) =>
        Run(() =>
        {
            action();
            return Task.FromResult(0);
        });
}
