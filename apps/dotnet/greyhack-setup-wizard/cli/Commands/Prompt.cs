using GreyHackSetupWizard.Cli.Resources;

namespace GreyHackSetupWizard.Cli.Commands;

internal static class Prompt
{
    public static bool Confirm(string question, bool yes)
    {
        if (yes)
        {
            return true;
        }
        if (Console.IsInputRedirected)
        {
            return false;
        }
        Console.Write($"{question} {CommandText.PromptChoices} ");
        string? reply = Console.ReadLine();
        return reply != null
            && reply
                .Trim()
                .StartsWith(CommandText.PromptYes, StringComparison.CurrentCultureIgnoreCase);
    }
}
