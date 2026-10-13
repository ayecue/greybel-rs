using Avalonia;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Gui.ViewModels;
using GreyHackSetupWizard.Gui.Views;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui;

public sealed class App : Application
{
    private HttpClient? http;

    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            http = new HttpClient();
            desktop.Exit += (_, _) => http.Dispose();
            var window = new MainWindow();
            var services = new WizardServiceContainer(
                new StorageFilePicker(window),
                new DownloadSettings(http, DownloadSettings.GetDefaultCacheFolder()),
                GameLocator.Locate,
                PluginRelease.DefaultRepository
            );
            var main = new MainViewModel(services);
            window.DataContext = main;
            desktop.MainWindow = window;
            window.Opened += async (_, _) => await main.StartAsync();
        }
        base.OnFrameworkInitializationCompleted();
    }
}
