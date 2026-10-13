using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net;
using Duplex.Tests.Calculator;
using Duplex.Tests.Support;
using static Duplex.Tests.Support.RustClientSetup;

namespace Duplex.Tests;

public class RustClientTests
{
    [Fact]
    public async Task Should_serve_the_rust_core_over_tcp()
    {
        var hub = new Hub();
        hub.Start();
        var logTexts = new BlockingCollection<string>();
        var closeReasons = new BlockingCollection<ErrorCode>();
        using var provider = new TcpProvider(
            hub,
            new IPEndPoint(IPAddress.Loopback, 0),
            driver =>
            {
                CalculatorChannel? channel = null;
                channel = new CalculatorChannel(
                    driver.Transport,
                    new Responder(hub, logTexts, () => channel!),
                    new ChannelOptions()
                );
                channel.OnClose += closeReasons.Add;
            }
        );
        provider.Start();
        var start = new ProcessStartInfo(
            "cargo",
            [
                "run",
                "--quiet",
                "-p",
                "duplex-driver-stdio",
                "--example",
                "tcp-user",
                "--",
                provider.Endpoint.Port.ToString(),
            ]
        )
        {
            WorkingDirectory = Path.Combine(Scenario.Duplex, "rs"),
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };

        using var user = Process.Start(start)!;
        var cancellation = TestContext.Current.CancellationToken;
        var output = user.StandardOutput.ReadToEndAsync(cancellation);
        var errors = user.StandardError.ReadToEndAsync(cancellation);
        var exited = user.WaitForExit(WaitLimit);
        if (!exited)
        {
            user.Kill();
        }
        var reason = closeReasons.TryTake(out var code, TimeSpan.FromSeconds(10))
            ? code
            : (ErrorCode?)null;
        hub.Stop();

        Assert.True(exited && user.ExitCode == 0, await errors);
        Assert.Equal(
            ["Active", "ClosedChannel"],
            (await output).Split('\n', StringSplitOptions.RemoveEmptyEntries)
        );
        Assert.Equal(["from rust"], logTexts);
        Assert.Equal(ErrorCode.ClosedChannel, reason);
    }
}
