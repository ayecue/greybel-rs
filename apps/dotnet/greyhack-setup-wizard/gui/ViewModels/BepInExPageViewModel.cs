using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class BepInExPageViewModel(
    GameStatusViewModel game,
    WizardServiceContainer services,
    Platform platform
) : WizardPage
{
    private string? choiceFolder;

    public override string Title => WindowText.BepInExPageTitle;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem), nameof(IsInstallMode))]
    public partial SetupAction Action { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(ReleaseText), nameof(IsRosettaNoteVisible))]
    public partial BepInExMajor Major { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem))]
    public partial SourceKind Source { get; set; }

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(Problem))]
    public partial string? ArchivePath { get; set; }

    public bool IsInstallMode => Action == SetupAction.Install;

    public string CurrentText => Localization.Format(WindowText.InstalledValue, game.BepInExText);

    public bool IsRemovable => game.Status?.BepInEx != null;

    public BepInExRelease Release => BepInExRelease.Get(Major, platform);

    public string ReleaseText =>
        Localization.Format(WindowText.BepInExReleaseValue, Release.Version, Release.Url);

    public bool IsRosettaNoteVisible => Major == BepInExMajor.Beta && platform == Platform.MacOS;

    public override string? Problem =>
        Action switch
        {
            SetupAction.Install when Source == SourceKind.File && ArchivePath == null =>
                WindowText.BepInExZipMissing,
            SetupAction.Remove when !IsRemovable => WindowText.BepInExNothingToRemove,
            _ => null,
        };

    public BepInExMajor? GetResultMajor() =>
        Action switch
        {
            SetupAction.Install => Major,
            SetupAction.Remove => null,
            _ => game.Status?.BepInEx?.Major,
        };

    public override Task EnterAsync()
    {
        string? folder = game.Status?.Game.Folder;
        if (folder != choiceFolder)
        {
            choiceFolder = folder;
            BepInExInstallation? installation = game.Status?.BepInEx;
            Action = installation == null ? SetupAction.Install : SetupAction.Skip;
            Major = installation?.Major ?? BepInExMajor.Stable;
            Source = SourceKind.Download;
            ArchivePath = null;
        }
        OnPropertyChanged(nameof(CurrentText));
        OnPropertyChanged(nameof(IsRemovable));
        OnPropertyChanged(nameof(Problem));
        return Task.CompletedTask;
    }

    [RelayCommand]
    private async Task ChooseArchiveAsync()
    {
        string? archive = await services.Picker.PickFileAsync(
            WindowText.ZipPickerTitle,
            WindowText.ZipFileKind,
            ["*.zip"]
        );
        if (archive != null)
        {
            ArchivePath = archive;
            Source = SourceKind.File;
        }
    }
}
