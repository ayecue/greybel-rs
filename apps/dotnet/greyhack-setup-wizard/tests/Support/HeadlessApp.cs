using Avalonia;
using Avalonia.Headless;
using GreyHackSetupWizard.Gui;

[assembly: AvaloniaTestApplication(typeof(GreyHackSetupWizard.Tests.Support.HeadlessApp))]

namespace GreyHackSetupWizard.Tests.Support;

public static class HeadlessApp
{
    private static readonly HeadlessUnitTestSession Session =
        HeadlessUnitTestSession.GetOrStartForAssembly(typeof(HeadlessApp).Assembly);

    public static Task RunAsync(Func<Task> test) =>
        Session.Dispatch(
            async () =>
            {
                await test();
                return true;
            },
            TestContext.Current.CancellationToken
        );

    public static AppBuilder BuildAvaloniaApp() =>
        AppBuilder
            .Configure<App>()
            .UseSkia()
            .WithInterFont()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = false });
}
