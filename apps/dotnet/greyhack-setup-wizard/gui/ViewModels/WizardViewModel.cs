using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class WizardViewModel : ObservableObject
{
    private readonly List<WizardPage> pages;
    private readonly Action close;

    public WizardViewModel(
        GameStatusViewModel game,
        WizardServiceContainer services,
        Platform platform,
        Action close
    )
    {
        this.close = close;
        BepInEx = new BepInExPageViewModel(game, services, platform);
        Plugin = new PluginPageViewModel(game, BepInEx, services);
        Port = new PortPageViewModel(game);
        Summary = new SummaryPageViewModel(game, BepInEx, Plugin, Port, services);
        pages = [BepInEx, Plugin, Port, Summary];
        foreach (WizardPage page in pages)
        {
            page.PropertyChanged += HandlePagePropertyChange;
        }
        CurrentPage = BepInEx;
        BepInEx.IsCurrent = true;
    }

    public BepInExPageViewModel BepInEx { get; }

    public PluginPageViewModel Plugin { get; }

    public PortPageViewModel Port { get; }

    public SummaryPageViewModel Summary { get; }

    public IReadOnlyList<WizardPage> Pages => pages;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsLastPage), nameof(StepText))]
    [NotifyCanExecuteChangedFor(nameof(GoBackCommand), nameof(GoNextCommand))]
    public partial WizardPage CurrentPage { get; set; }

    public bool IsLastPage => CurrentPage == Summary;

    public string StepText =>
        Localization.Format(WindowText.StepCounter, GetIndex() + 1, Pages.Count);

    public string CloseText => Summary.IsDone ? WindowText.Done : WindowText.Cancel;

    public Task StartAsync() => CurrentPage.EnterAsync();

    private int GetIndex() => pages.IndexOf(CurrentPage);

    private bool CanGoBack() => GetIndex() > 0 && !Summary.IsRunning && !Summary.IsDone;

    private bool CanGoNext() => !IsLastPage && CurrentPage.Problem == null;

    private bool CanClose() => !Summary.IsRunning;

    [RelayCommand(CanExecute = nameof(CanGoBack))]
    private Task GoBackAsync() => TurnToAsync(GetIndex() - 1);

    [RelayCommand(CanExecute = nameof(CanGoNext))]
    private Task GoNextAsync() => TurnToAsync(GetIndex() + 1);

    [RelayCommand(CanExecute = nameof(CanClose))]
    private void Close() => close();

    private async Task TurnToAsync(int index)
    {
        CurrentPage.IsCurrent = false;
        CurrentPage = pages[index];
        CurrentPage.IsCurrent = true;
        await CurrentPage.EnterAsync();
    }

    private void HandlePagePropertyChange(object? sender, PropertyChangedEventArgs e)
    {
        GoBackCommand.NotifyCanExecuteChanged();
        GoNextCommand.NotifyCanExecuteChanged();
        CloseCommand.NotifyCanExecuteChanged();
        OnPropertyChanged(nameof(CloseText));
    }
}
