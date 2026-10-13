using System.CommandLine;
using GreyHackSetupWizard;
using GreyHackSetupWizard.Cli.Commands;
using GreyHackSetupWizard.Cli.Resources;

try
{
    Localization.Apply(Localization.FindLanguage(args));
}
catch (SetupException error)
{
    Console.Error.WriteLine(Localization.Format(CommandText.ErrorLine, error.Message));
    return 1;
}

var root = new RootCommand(CommandText.RootDescription)
{
    Options.CreateLanguageOption(),
    LocateCommand.Create(),
    BepInExCommand.Create(),
    PluginCommand.Create(),
    ConfigureCommand.Create(),
};
return root.Parse(args).Invoke();
