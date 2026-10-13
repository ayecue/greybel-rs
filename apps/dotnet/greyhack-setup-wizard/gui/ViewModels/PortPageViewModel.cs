using CommunityToolkit.Mvvm.ComponentModel;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Plugins;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class PortPageViewModel : WizardPage
{
    private readonly GameStatusViewModel game;

    private string? choiceFolder;

    public PortPageViewModel(GameStatusViewModel game)
    {
        this.game = game;
        PropertyChanged += (_, e) =>
        {
            if (e.PropertyName is nameof(Port) or nameof(Action))
            {
                UpdatePortWarning();
            }
        };
    }

    public override string Title => WindowText.PortPageTitle;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem), nameof(IsPortSetup))]
    public partial SetupAction Action { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem))]
    public partial decimal? Port { get; set; }

    public bool IsPortSetup => Action == SetupAction.Install;

    [ObservableProperty]
    public partial string? PortWarning { get; set; }

    public string CurrentText => Localization.Format(WindowText.InstalledValue, game.PortText);

    public override string? Problem =>
        IsPortSetup && GetPort() == null ? WindowText.PortInvalid : null;

    public int? GetPort() =>
        Port is { } port && port == decimal.Truncate(port) && port is >= 1 and <= 65535
            ? (int)port
            : null;

    private void UpdatePortWarning()
    {
        int? port = IsPortSetup ? GetPort() : game.Status?.Port ?? PluginConfig.DefaultPort;
        PortWarning =
            port is { } number && PortUsage.IsInUse(number)
                ? Localization.Format(WindowText.PortInUse, number)
                : null;
    }

    public override Task EnterAsync()
    {
        string? folder = game.Status?.Game.Folder;
        if (folder != choiceFolder)
        {
            choiceFolder = folder;
            int? currentPort = game.Status?.Port;
            Action = currentPort == null ? SetupAction.Install : SetupAction.Skip;
            Port = currentPort ?? PluginConfig.DefaultPort;
        }
        OnPropertyChanged(nameof(CurrentText));
        UpdatePortWarning();
        return Task.CompletedTask;
    }
}
