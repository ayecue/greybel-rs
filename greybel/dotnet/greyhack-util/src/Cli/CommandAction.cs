using GreyHackUtil.Contracts;

namespace GreyHackUtil.Cli;

internal static class CommandAction
{
    public static int Run(Action action)
    {
        try
        {
            action();
            return 0;
        }
        catch (Exception error)
            when (error
                    is ContractException
                        or InvalidDataException
                        or IOException
                        or HttpRequestException
            )
        {
            Console.Error.WriteLine($"error: {error.Message}");
            return 1;
        }
    }
}
