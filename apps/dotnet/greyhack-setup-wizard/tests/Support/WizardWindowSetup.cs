using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Avalonia.VisualTree;
using CommunityToolkit.Mvvm.Input;
using GreyHackSetupWizard.Gui.Views;
using GreyHackSetupWizard.Plugins;

namespace GreyHackSetupWizard.Tests.Support;

internal static class WizardWindowSetup
{
    internal static async Task Click(Button button)
    {
        Assert.True(button.IsEffectivelyEnabled, $"{button.Content} is disabled");
        if (button.Command is IAsyncRelayCommand command)
        {
            await command.ExecuteAsync(null);
        }
        else
        {
            button.Command!.Execute(null);
        }
        Dispatcher.UIThread.RunJobs();
    }

    internal static T FindNamed<T>(Window window, string name)
        where T : Control =>
        window
            .GetVisualDescendants()
            .OfType<T>()
            .Single(control => control.Name == name && control.IsEffectivelyVisible);

    internal static T Find<T>(Window window, string content)
        where T : ContentControl =>
        window
            .GetVisualDescendants()
            .OfType<T>()
            .Single(control => Equals(control.Content, content) && control.IsEffectivelyVisible);

    internal static string GetHeading(Window window) =>
        window
            .GetVisualDescendants()
            .OfType<TextBlock>()
            .First(text => text.Classes.Contains("heading") && text.IsEffectivelyVisible)
            .Text!;

    internal static void SaveScreenshot(Window window, string name)
    {
        string? folder = Environment.GetEnvironmentVariable("WIZARD_SCREENSHOTS");
        if (string.IsNullOrEmpty(folder))
        {
            return;
        }
        Directory.CreateDirectory(folder);
        Dispatcher.UIThread.RunJobs();
        window
            .CaptureRenderedFrame()
            ?.Save(Path.Combine(folder, name + ".png"), new PngBitmapEncoderOptions());
    }
}
