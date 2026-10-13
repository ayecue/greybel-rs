using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui.ViewModels;

public sealed partial class SummaryPageViewModel(
    GameStatusViewModel game,
    BepInExPageViewModel bepInEx,
    PluginPageViewModel plugin,
    PortPageViewModel port,
    WizardServiceContainer services
) : WizardPage
{
    public override string Title => WindowText.SummaryPageTitle;

    public override string? Problem => null;

    public ObservableCollection<string> Changes { get; } = [];

    public ObservableCollection<string> Log { get; } = [];

    [ObservableProperty]
    [NotifyCanExecuteChangedFor(nameof(ApplyCommand))]
    public partial bool IsRunning { get; set; }

    [ObservableProperty]
    [NotifyCanExecuteChangedFor(nameof(ApplyCommand))]
    public partial bool IsDone { get; set; }

    [ObservableProperty]
    public partial string? Outcome { get; set; }

    [ObservableProperty]
    public partial bool IsFailure { get; set; }

    public bool HasChanges => Changes.Count > 0;

    public override Task EnterAsync()
    {
        Changes.Clear();
        foreach (string change in DescribeChanges())
        {
            Changes.Add(change);
        }
        OnPropertyChanged(nameof(HasChanges));
        ApplyCommand.NotifyCanExecuteChanged();
        return Task.CompletedTask;
    }

    private IEnumerable<string> DescribeChanges()
    {
        switch (bepInEx.Action)
        {
            case SetupAction.Install when bepInEx.Source == SourceKind.Download:
                yield return Localization.Format(
                    WindowText.ChangeBepInExDownload,
                    bepInEx.Release.Version
                );
                break;
            case SetupAction.Install:
                yield return Localization.Format(WindowText.ChangeBepInExZip, bepInEx.ArchivePath);
                break;
            case SetupAction.Remove:
                yield return WindowText.ChangeBepInExRemoval;
                break;
        }
        switch (plugin.Action)
        {
            case SetupAction.Install
                when plugin.Source == SourceKind.Download && plugin.Major is { } major:
                yield return Localization.Format(
                    WindowText.ChangePluginDownload,
                    PluginBuild.MessageHook.GetAssemblyName(major) + ".dll",
                    plugin.Release?.Tag
                );
                break;
            case SetupAction.Install:
                yield return Localization.Format(WindowText.ChangePluginFile, plugin.AssemblyPath);
                break;
            case SetupAction.Remove:
                yield return WindowText.ChangePluginRemoval;
                break;
        }
        if (port.IsPortSetup && port.GetPort() is { } number)
        {
            yield return Localization.Format(WindowText.ChangePort, number);
        }
    }

    private bool CanApply() => HasChanges && !IsRunning && !IsDone;

    [RelayCommand(CanExecute = nameof(CanApply))]
    private async Task ApplyAsync()
    {
        GameLayout layout = game.Status!.Game;
        var progress = new LogProgress(Log);
        IsRunning = true;
        try
        {
            BepInExMajor? major = bepInEx.GetResultMajor();
            if (bepInEx.Action == SetupAction.Install)
            {
                string? archive = bepInEx.Source == SourceKind.File ? bepInEx.ArchivePath : null;
                BepInExInstallation installation = await BepInExSetup.InstallAsync(
                    layout,
                    bepInEx.Major,
                    archive,
                    services.Download,
                    progress
                );
                major = installation.Major;
            }
            else if (bepInEx.Action == SetupAction.Remove)
            {
                BepInExSetup.Remove(layout, progress);
            }
            if (plugin.Action == SetupAction.Install)
            {
                PluginSource source =
                    plugin.Source == SourceKind.File
                        ? PluginSource.FromAssembly(plugin.AssemblyPath!)
                        : PluginSource.FromRelease(plugin.Release!);
                await PluginSetup.InstallAsync(
                    layout,
                    major!.Value,
                    source,
                    services.Download,
                    progress
                );
            }
            else if (plugin.Action == SetupAction.Remove)
            {
                PluginSetup.Remove(layout, progress);
            }
            if (port.IsPortSetup)
            {
                PluginSetup.ConfigurePort(layout, port.GetPort()!.Value, progress);
            }
            Outcome = WindowText.OutcomeComplete;
        }
        catch (Exception error)
            when (error
                    is SetupException
                        or IOException
                        or UnauthorizedAccessException
                        or InvalidDataException
            )
        {
            Log.Add(Localization.Format(WindowText.LogFailure, error.Message));
            Outcome = Localization.Format(WindowText.OutcomeFailure, error.Message);
            IsFailure = true;
        }
        finally
        {
            IsRunning = false;
            IsDone = true;
            game.Refresh();
        }
    }
}
