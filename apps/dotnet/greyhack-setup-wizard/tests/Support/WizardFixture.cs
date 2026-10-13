using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Gui.Services;
using GreyHackSetupWizard.Gui.ViewModels;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Tests.Support;

internal sealed class WizardFixture : IDisposable
{
    private readonly HttpClient http;

    public WizardFixture(bool isGameFound = true)
    {
        Game = FakeGame.Create(Folder);
        http = new HttpClient(GitHub);
        var services = new WizardServiceContainer(
            Picker,
            new DownloadSettings(http, Folder.GetPath("cache")),
            () => isGameFound ? Game : null,
            FakeGitHub.Repository
        );
        Main = new MainViewModel(services, Platform.MacOS);
    }

    public TemporaryFolder Folder { get; } = new();

    public GameLayout Game { get; }

    public FakeFilePicker Picker { get; } = new();

    public FakeGitHub GitHub { get; } = new();

    public MainViewModel Main { get; }

    public async Task<WizardViewModel> OpenWizardAsync()
    {
        await Main.StartAsync();
        await Main.StartSetupCommand.ExecuteAsync(null);
        return Main.Wizard!;
    }

    public string CreateFile(string name, string content = "")
    {
        string path = Folder.GetPath(name);
        File.WriteAllText(path, content);
        return path;
    }

    public string CreateStableArchive() =>
        FakeGame.CreateArchive(Folder, "stable.zip", "run_bepinex.sh", "BepInEx/core/BepInEx.dll");

    public void Dispose()
    {
        http.Dispose();
        Folder.Dispose();
    }
}
