using System.Net;
using System.Net.Sockets;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Gui.ViewModels;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class WizardViewModelTests
{
    [Fact]
    public async Task ShouldShowTheStatusOfTheGameFound()
    {
        using var fixture = new WizardFixture();
        MainViewModel main = fixture.Main;

        await main.StartAsync();

        Assert.False(main.IsWizardOpen);
        Assert.Equal(fixture.Game.Folder, main.Game.FolderText);
        Assert.Equal(
            ("Not installed", "Not installed"),
            (main.Game.BepInExText, main.Game.PluginText)
        );
        Assert.Equal("Not ready: BepInEx isn't installed.", main.Game.ReadinessText);
        Assert.Equal("Set up…", main.SetupText);
        Assert.True(main.StartSetupCommand.CanExecute(null));
    }

    [Fact]
    public async Task ShouldReportAPluginBuildForTheOtherBepInEx()
    {
        using var fixture = new WizardFixture();
        FakeGame.Write(fixture.Game, "BepInEx/core/BepInEx.dll");
        FakeGame.Write(fixture.Game, "BepInEx/plugins/GreyHackMessageHook.dll");

        await fixture.Main.StartAsync();

        Assert.False(fixture.Main.Game.IsReady);
        Assert.Equal(
            "Not ready: GreyHackMessageHook.dll is the plugin file for another BepInEx version. BepInEx 5 needs GreyHackMessageHook5.dll.",
            fixture.Main.Game.ReadinessText
        );
    }

    [Fact]
    public async Task ShouldOfferToInstallWhatTheGameLacks()
    {
        using var fixture = new WizardFixture();
        fixture.GitHub.AddFile("GreyHackMessageHook5.dll", [1, 2, 3]);
        WizardViewModel wizard = await fixture.OpenWizardAsync();

        await wizard.GoNextCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);

        Assert.True(fixture.Main.IsWizardOpen);
        Assert.Equal(
            (SetupAction.Install, BepInExMajor.Stable, SourceKind.Download),
            (wizard.BepInEx.Action, wizard.BepInEx.Major, wizard.BepInEx.Source)
        );
        Assert.Equal(
            (SetupAction.Install, SourceKind.Download),
            (wizard.Plugin.Action, wizard.Plugin.Source)
        );
        Assert.Equal($"{FakeGitHub.Tag} from {FakeGitHub.Repository}", wizard.Plugin.ReleaseText);
        Assert.Equal((SetupAction.Install, 7777m), (wizard.Port.Action, wizard.Port.Port));
        Assert.Same(wizard.Port, wizard.CurrentPage);
    }

    [Fact]
    public async Task ShouldInstallWhatThePagesChoseAndShowTheGameReady()
    {
        using var fixture = new WizardFixture();
        fixture.GitHub.AddFile("GreyHackMessageHook5.dll", [1, 2, 3]);
        WizardViewModel wizard = await fixture.OpenWizardAsync();
        fixture.Picker.Enqueue(fixture.CreateStableArchive());
        await wizard.BepInEx.ChooseArchiveCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);
        wizard.Port.Port = 8888;
        await wizard.GoNextCommand.ExecuteAsync(null);

        await wizard.Summary.ApplyCommand.ExecuteAsync(null);
        string closeText = wizard.CloseText;
        wizard.CloseCommand.Execute(null);

        Assert.Equal(3, wizard.Summary.Changes.Count);
        Assert.Equal("Installation complete.", wizard.Summary.Outcome);
        Assert.Equal("Done", closeText);
        Assert.Equal(
            [1, 2, 3],
            File.ReadAllBytes(Path.Combine(fixture.Game.PluginsFolder, "GreyHackMessageHook5.dll"))
        );
        Assert.Equal(8888, PluginConfig.GetPort(fixture.Game));
        Assert.False(fixture.Main.IsWizardOpen);
        Assert.Equal(
            "Setup is already done. Message Hook connects on port 8888.",
            fixture.Main.Game.ReadinessText
        );
        Assert.Equal("Change setup…", fixture.Main.SetupText);
    }

    [Fact]
    public async Task ShouldChangeNothingWhenTheWizardIsCancelled()
    {
        using var fixture = new WizardFixture();
        WizardViewModel wizard = await fixture.OpenWizardAsync();
        fixture.Picker.Enqueue(fixture.CreateStableArchive());
        await wizard.BepInEx.ChooseArchiveCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);

        string closeText = wizard.CloseText;
        wizard.CloseCommand.Execute(null);

        Assert.Equal("Cancel", closeText);
        Assert.False(fixture.Main.IsWizardOpen);
        Assert.False(Directory.Exists(fixture.Game.BepInExFolder));
        Assert.True(fixture.Main.StartSetupCommand.CanExecute(null));
    }

    [Fact]
    public async Task ShouldWarnThatAnotherProgramUsesThePort()
    {
        using var other = new TcpListener(IPAddress.Loopback, 0);
        other.Start();
        int port = ((IPEndPoint)other.LocalEndpoint).Port;
        using var fixture = new WizardFixture();
        WizardViewModel wizard = await fixture.OpenWizardAsync();
        await wizard.GoNextCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);

        wizard.Port.Port = port;

        Assert.Equal(
            $"The port {port} the message hook requires is already used.",
            wizard.Port.PortWarning
        );
        Assert.True(wizard.GoNextCommand.CanExecute(null));
    }

    [Fact]
    public async Task ShouldStopAtABuildForTheOtherBepInEx()
    {
        using var fixture = new WizardFixture();
        WizardViewModel wizard = await fixture.OpenWizardAsync();
        await wizard.GoNextCommand.ExecuteAsync(null);
        fixture.Picker.Enqueue(fixture.CreateFile("GreyHackMessageHook.dll"));

        await wizard.Plugin.ChooseAssemblyCommand.ExecuteAsync(null);

        Assert.Equal(
            "GreyHackMessageHook.dll is the wrong file. BepInEx 5 needs GreyHackMessageHook5.dll.",
            wizard.Plugin.Problem
        );
        Assert.False(wizard.GoNextCommand.CanExecute(null));
    }

    [Fact]
    public async Task ShouldOfferABuildTheUserHasWhenThereIsNoRelease()
    {
        using var fixture = new WizardFixture();
        WizardViewModel wizard = await fixture.OpenWizardAsync();

        await wizard.GoNextCommand.ExecuteAsync(null);

        Assert.False(wizard.Plugin.IsReleaseAvailable);
        Assert.Equal(SourceKind.File, wizard.Plugin.Source);
        Assert.Equal(
            "Choose GreyHackMessageHook5.dll, the plugin file for BepInEx 5.",
            wizard.Plugin.Problem
        );
    }

    [Fact]
    public async Task ShouldOpenTheFolderTheUserChoosesWhenNoGameIsFound()
    {
        using var fixture = new WizardFixture(isGameFound: false);
        MainViewModel main = fixture.Main;
        await main.StartAsync();
        bool couldStart = main.StartSetupCommand.CanExecute(null);
        fixture.Picker.Enqueue(fixture.Game.Folder);

        await main.Game.ChooseFolderCommand.ExecuteAsync(null);

        Assert.False(couldStart);
        Assert.Equal(fixture.Game.Folder, main.Game.FolderText);
        Assert.Null(main.Game.Error);
        Assert.True(main.StartSetupCommand.CanExecute(null));
    }

    [Fact]
    public async Task ShouldReportAStepThatFails()
    {
        using var fixture = new WizardFixture();
        WizardViewModel wizard = await fixture.OpenWizardAsync();
        fixture.Picker.Enqueue(FakeGame.CreateArchive(fixture.Folder, "other.zip", "readme.txt"));
        await wizard.BepInEx.ChooseArchiveCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);
        wizard.Plugin.Action = SetupAction.Skip;
        await wizard.GoNextCommand.ExecuteAsync(null);
        await wizard.GoNextCommand.ExecuteAsync(null);

        await wizard.Summary.ApplyCommand.ExecuteAsync(null);

        Assert.True(wizard.Summary.IsFailure);
        Assert.StartsWith("Unexpected failure: ", wizard.Summary.Outcome);
        Assert.EndsWith(
            "isn't a BepInEx zip file (it has no BepInEx/core folder).",
            wizard.Summary.Outcome
        );
        Assert.Null(PluginConfig.GetPort(fixture.Game));
    }

    [Fact]
    public async Task ShouldChangeNothingForAGameThatHasEverything()
    {
        using var fixture = new WizardFixture();
        FakeGame.Write(fixture.Game, "BepInEx/core/BepInEx.dll");
        FakeGame.Write(fixture.Game, "BepInEx/plugins/GreyHackMessageHook5.dll");
        PluginConfig.SetPort(fixture.Game, 7777);
        WizardViewModel wizard = await fixture.OpenWizardAsync();

        for (int page = 1; page < wizard.Pages.Count; page++)
        {
            await wizard.GoNextCommand.ExecuteAsync(null);
        }

        Assert.True(fixture.Main.Game.IsReady);
        Assert.Equal(
            (SetupAction.Skip, SetupAction.Skip, SetupAction.Skip),
            (wizard.BepInEx.Action, wizard.Plugin.Action, wizard.Port.Action)
        );
        Assert.False(wizard.Summary.HasChanges);
        Assert.False(wizard.Summary.ApplyCommand.CanExecute(null));
    }
}
