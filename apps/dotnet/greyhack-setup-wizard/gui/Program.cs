using Avalonia;

namespace GreyHackSetupWizard.Gui;

internal static class Program
{
    [STAThread]
    public static int Main(string[] args)
    {
        try
        {
            Localization.Apply(Localization.FindLanguage(args));
        }
        catch (SetupException error)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
        return BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
    }

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder.Configure<App>().UsePlatformDetect().WithInterFont().LogToTrace();
}
