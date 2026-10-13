using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class GameStatusViewModel(WizardServiceContainer services) : ObservableObject
{
    [ObservableProperty]
    [NotifyPropertyChangedFor(
        nameof(HasGame),
        nameof(FolderText),
        nameof(BepInExText),
        nameof(PluginText),
        nameof(PortText),
        nameof(IsReady),
        nameof(ReadinessText)
    )]
    public partial GameStatus? Status { get; set; }

    [ObservableProperty]
    public partial string? Error { get; set; }

    public bool HasGame => Status != null;

    public string FolderText => Status?.Game.Folder ?? WindowText.StatusFolderMissing;

    public string BepInExText => Status?.BepInEx?.GetDescription() ?? WindowText.StatusNotInstalled;

    public string PluginText =>
        Status?.FindMessageHook() is { } assembly
            ? assembly + ".dll"
            : WindowText.StatusNotInstalled;

    public string PortText =>
        Status?.Port is { } port
            ? port.ToString()
            : Localization.Format(WindowText.StatusPortDefault, PluginConfig.DefaultPort);

    public bool IsReady =>
        Status?.BepInEx is { } bepInEx
        && Status.FindMessageHook() == PluginBuild.MessageHook.GetAssemblyName(bepInEx.Major);

    public string ReadinessText
    {
        get
        {
            if (Status == null)
            {
                return WindowText.ReadinessNoGame;
            }
            if (Status.BepInEx is not { } bepInEx)
            {
                return WindowText.ReadinessNoBepInEx;
            }
            string? plugin = Status.FindMessageHook();
            string build = PluginBuild.MessageHook.GetAssemblyName(bepInEx.Major);
            if (plugin == null)
            {
                return WindowText.ReadinessNoPlugin;
            }
            if (plugin != build)
            {
                return Localization.Format(
                    WindowText.ReadinessWrongPlugin,
                    plugin + ".dll",
                    (int)bepInEx.Major,
                    build + ".dll"
                );
            }
            return Localization.Format(
                WindowText.ReadinessReady,
                Status.Port ?? PluginConfig.DefaultPort
            );
        }
    }

    public void Refresh()
    {
        if (Status != null)
        {
            Status = GameStatus.Read(Status.Game);
        }
    }

    [RelayCommand]
    public void Locate()
    {
        try
        {
            GameLayout? game = services.LocateGame();
            if (game == null)
            {
                Error = WindowText.GameNotFound;
                return;
            }
            ShowGame(game);
        }
        catch (SetupException error)
        {
            Error = error.Message;
        }
    }

    [RelayCommand]
    private async Task ChooseFolderAsync()
    {
        string? folder = await services.Picker.PickFolderAsync(WindowText.FolderPickerTitle);
        if (folder == null)
        {
            return;
        }
        try
        {
            ShowGame(GameLayout.FromFolder(folder));
        }
        catch (SetupException error)
        {
            Status = null;
            Error = error.Message;
        }
    }

    private void ShowGame(GameLayout game)
    {
        Error = null;
        Status = GameStatus.Read(game);
    }
}
