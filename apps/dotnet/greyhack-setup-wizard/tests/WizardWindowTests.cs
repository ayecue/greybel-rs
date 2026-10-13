using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.Gui.Views;
using GreyHackSetupWizard.Plugins;
using GreyHackSetupWizard.Tests.Support;
using static GreyHackSetupWizard.Tests.Support.WizardWindowSetup;

namespace GreyHackSetupWizard.Tests;

public class WizardWindowTests
{
    [Fact]
    public Task ShouldGoFromTheStatusThroughTheWizardAndBack() =>
        HeadlessApp.RunAsync(async () =>
        {
            using var fixture = new WizardFixture();
            fixture.GitHub.AddFile("GreyHackMessageHook5.dll", [1, 2, 3]);
            fixture.Picker.Enqueue(fixture.CreateStableArchive());
            var window = new MainWindow { DataContext = fixture.Main };
            window.Show();
            await fixture.Main.StartAsync();
            Dispatcher.UIThread.RunJobs();
            var headings = new List<string> { GetHeading(window) };
            SaveScreenshot(window, "1-Status");

            await Click(window.FindControl<Button>("startSetup")!);
            Button next = FindNamed<Button>(window, "next");
            int pageCount = fixture.Main.Wizard!.Pages.Count;
            for (int page = 0; page < pageCount; page++)
            {
                if (page == 0)
                {
                    await Click(Find<Button>(window, "Choose file…"));
                }
                headings.Add(GetHeading(window));
                SaveScreenshot(window, $"{page + 2}-{fixture.Main.Wizard.CurrentPage.Title}");
                if (page < pageCount - 1)
                {
                    await Click(next);
                }
            }
            await Click(Find<Button>(window, "Apply"));
            SaveScreenshot(window, $"{pageCount + 2}-Applied");
            string outcome = FindNamed<TextBlock>(window, "outcome").Text!;
            await Click(FindNamed<Button>(window, "close"));
            headings.Add(GetHeading(window));
            SaveScreenshot(window, $"{pageCount + 3}-Status");

            Assert.Equal(
                ["Grey Hack", "BepInEx", "Message Hook Plugin", "Port", "Summary", "Grey Hack"],
                headings
            );
            Assert.Equal("Installation complete.", outcome);
            Assert.Equal(
                "Setup is already done. Message Hook connects on port 7777.",
                FindNamed<Border>(window, "readiness").Child is TextBlock text ? text.Text : null
            );
            Assert.Equal(PluginConfig.DefaultPort, PluginConfig.GetPort(fixture.Game));
            window.Close();
        });

    [Fact]
    public Task ShouldShowWhyThereIsNoGameAndNotStartTheWizard() =>
        HeadlessApp.RunAsync(async () =>
        {
            using var fixture = new WizardFixture(isGameFound: false);
            var window = new MainWindow { DataContext = fixture.Main };
            window.Show();

            await fixture.Main.StartAsync();
            Dispatcher.UIThread.RunJobs();
            SaveScreenshot(window, "0-No game");

            TextBlock error = FindNamed<TextBlock>(window, "error");
            Assert.True(error.IsVisible);
            Assert.StartsWith("Grey Hack wasn't found", error.Text);
            Assert.False(window.FindControl<Button>("startSetup")!.IsEffectivelyEnabled);
            window.Close();
        });
}
