using System.Collections.Concurrent;
using System.Net;
using System.Net.Sockets;
using Duplex.Tests.Calculator;
using Duplex.Tests.Support;
using static Duplex.Tests.Support.TcpSetup;

namespace Duplex.Tests;

public class TcpTests
{
    [Fact]
    public void Should_serve_a_client_over_tcp()
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
                var channel = new CalculatorChannel(
                    driver.Transport,
                    new Logger(logTexts),
                    new ChannelOptions { WithoutHealthCheck = true }
                );
                channel.OnClose += closeReasons.Add;
            }
        );
        provider.Start();
        using var client = new TcpClient();
        client.Connect(provider.Endpoint);
        var stream = client.GetStream();
        var channelId = Calculation.Definition.ChannelId;

        var listen = TestFrames.Decode(ReadBatch(stream));
        WriteBatch(
            stream,
            TestFrames.Encode(
                channelId,
                Role.Client,
                0,
                (_, encoder) =>
                    encoder.WriteControl(
                        new Control(FrameKind.Open)
                        {
                            Id = 7,
                            Core = new VersionNumber(1, 0),
                            Protocol = Calculation.Definition.Version,
                            Fingerprint = Calculation
                                .Definition.GetVersion(Calculation.Definition.Version.Minor)!
                                .Fingerprint,
                        }
                    )
            )
        );
        var ack = TestFrames.Decode(ReadBatch(stream));
        var connection = uint.Parse(ack.Frames[0].Split("connection ")[1].Split(',')[0]);
        WriteBatch(
            stream,
            TestFrames.Encode(
                channelId,
                Role.Client,
                connection,
                (writer, encoder) =>
                {
                    var body = new DuplexWriter();
                    new Log("over tcp").Write(body);
                    encoder.WriteEncodedMessage(Log.MessageType.Id, null, body.ToArray());
                }
            )
        );
        var log = logTexts.TryTake(out var text, WaitLimit) ? text : null;
        client.Close();
        var reason = closeReasons.TryTake(out var code, WaitLimit) ? code : (ErrorCode?)null;
        hub.Stop();

        Assert.Equal(["Listen"], listen.Frames);
        Assert.StartsWith("Ack(id 7, ", ack.Frames[0]);
        Assert.Equal("over tcp", log);
        Assert.Equal(ErrorCode.ConnectionLoss, reason);
    }
}
