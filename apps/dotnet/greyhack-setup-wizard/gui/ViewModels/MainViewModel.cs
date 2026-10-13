using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class MainViewModel : ObservableObject
{
    private readonly WizardServiceContainer services;
    private readonly Platform platform;

    public MainViewModel(WizardServiceContainer services)
        : this(services, PlatformDetector.Current) { }

    public MainViewModel(WizardServiceContainer services, Platform platform)
    {
        this.services = services;
        this.platform = platform;
        Game = new GameStatusViewModel(services);
        Game.PropertyChanged += (_, _) =>
        {
            OnPropertyChanged(nameof(SetupText));
            StartSetupCommand.NotifyCanExecuteChanged();
        };
    }

    public GameStatusViewModel Game { get; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsWizardOpen))]
    public partial WizardViewModel? Wizard { get; set; }

    public bool IsWizardOpen => Wizard != null;

    public string SetupText => Game.IsReady ? WindowText.SetupChange : WindowText.SetupStart;

    public Task StartAsync()
    {
        Game.Locate();
        return Task.CompletedTask;
    }

    private bool CanStartSetup() => Game.HasGame && Wizard == null;

    [RelayCommand(CanExecute = nameof(CanStartSetup))]
    private async Task StartSetupAsync()
    {
        Wizard = new WizardViewModel(Game, services, platform, CloseWizard);
        StartSetupCommand.NotifyCanExecuteChanged();
        await Wizard.StartAsync();
    }

    private void CloseWizard()
    {
        Game.Refresh();
        Wizard = null;
        StartSetupCommand.NotifyCanExecuteChanged();
    }
}
