using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class PluginPageViewModel(
    GameStatusViewModel game,
    BepInExPageViewModel bepInEx,
    WizardServiceContainer services
) : WizardPage
{
    private string? choiceFolder;
    private bool isReleaseQueried;

    public override string Title => WindowText.PluginPageTitle;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem), nameof(IsInstallMode))]
    public partial SetupAction Action { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem))]
    public partial SourceKind Source { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem))]
    public partial string? AssemblyPath { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem), nameof(IsReleaseAvailable))]
    public partial PluginRelease? Release { get; set; }

    [ObservableProperty]
    public partial string ReleaseText { get; set; } = WindowText.PluginReleaseLookup;

    public bool IsInstallMode => Action == SetupAction.Install;

    public bool IsReleaseAvailable => Release != null;

    public string CurrentText => Localization.Format(WindowText.InstalledValue, game.PluginText);

    public bool IsRemovable => game.Status?.FindMessageHook() != null;

    public BepInExMajor? Major => bepInEx.GetResultMajor();

    public string BuildText =>
        Major is { } major
            ? Localization.Format(
                WindowText.PluginFileValue,
                (int)major,
                PluginBuild.MessageHook.GetAssemblyName(major) + ".dll"
            )
            : WindowText.PluginWithoutBepInEx;

    public override string? Problem
    {
        get
        {
            if (Action == SetupAction.Remove)
            {
                return IsRemovable ? null : WindowText.PluginNothingToRemove;
            }
            if (Action != SetupAction.Install)
            {
                return null;
            }
            if (Major is not { } major)
            {
                return WindowText.PluginNeedsBepInEx;
            }
            string assemblyName = PluginBuild.MessageHook.GetAssemblyName(major) + ".dll";
            if (Source == SourceKind.File)
            {
                return AssemblyPath == null
                        ? Localization.Format(
                            WindowText.PluginFileMissing,
                            assemblyName,
                            (int)major
                        )
                    : !string.Equals(
                        Path.GetFileName(AssemblyPath),
                        assemblyName,
                        StringComparison.Ordinal
                    )
                        ? Localization.Format(
                            WindowText.PluginFileWrong,
                            Path.GetFileName(AssemblyPath),
                            (int)major,
                            assemblyName
                        )
                    : null;
            }
            if (Release == null)
            {
                return WindowText.PluginReleaseUnavailable;
            }
            try
            {
                Release.GetAsset(PluginBuild.MessageHook, major);
                return null;
            }
            catch (SetupException error)
            {
                return error.Message;
            }
        }
    }

    public override async Task EnterAsync()
    {
        string? folder = game.Status?.Game.Folder;
        if (folder != choiceFolder)
        {
            choiceFolder = folder;
            Action = IsRemovable ? SetupAction.Skip : SetupAction.Install;
            AssemblyPath = null;
            Source = Release != null ? SourceKind.Download : SourceKind.File;
        }
        OnPropertyChanged(nameof(CurrentText));
        OnPropertyChanged(nameof(IsRemovable));
        OnPropertyChanged(nameof(Major));
        OnPropertyChanged(nameof(BuildText));
        OnPropertyChanged(nameof(Problem));
        if (!isReleaseQueried)
        {
            isReleaseQueried = true;
            await FindReleaseAsync();
        }
    }

    private async Task FindReleaseAsync()
    {
        try
        {
            Release = await PluginSetup.FindReleaseAsync(
                services.Download,
                services.Repository,
                null
            );
            ReleaseText = Localization.Format(
                WindowText.PluginReleaseValue,
                Release.Tag,
                services.Repository
            );
            if (AssemblyPath == null)
            {
                Source = SourceKind.Download;
            }
        }
        catch (SetupException error)
        {
            ReleaseText = Localization.Format(WindowText.PluginReleaseMissing, error.Message);
            Source = SourceKind.File;
        }
    }

    [RelayCommand]
    private async Task ChooseAssemblyAsync()
    {
        string? assembly = await services.Picker.PickFileAsync(
            WindowText.PluginPickerTitle,
            WindowText.PluginFileKind,
            ["*.dll"]
        );
        if (assembly != null)
        {
            AssemblyPath = assembly;
            Source = SourceKind.File;
        }
    }
}
