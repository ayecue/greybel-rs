namespace GreyHackTestSupport.Cli;

internal static class CommandAction
{
    public static int Run(Func<int> action)
    {
        try
        {
            return action();
        }
        catch (Exception error)
            when (error is TestSupportException or GreyHackSetupWizard.SetupException or IOException
            )
        {
            Console.Error.WriteLine($"error: {error.Message}");
            return 1;
        }
    }
}
