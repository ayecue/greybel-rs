using GreyHackMessageHook.Game;
using NetworkMessages;
using Newtonsoft.Json.Linq;

namespace GreyHackMessageHook.Tests;

public class GameFrameTests
{
    [Theory]
    [InlineData(1)]
    [InlineData(17)]
    [InlineData(23)]
    [InlineData(24)]
    [InlineData(1_000_000)]
    [InlineData(4_000_000)]
    public void ShouldFrameAndReadMessagesAtTransportBoundaries(int size)
    {
        string text = new('x', size);
        var raw = new RawMessage(2013);
        raw.Add(text);

        byte[] frame = RpcFrame.Serialize(raw.ToObject<JObject>());
        int payloadLength = frame.Length - 4;
        Assert.Equal((byte)(payloadLength >> 24), frame[0]);
        Assert.Equal((byte)(payloadLength >> 16), frame[1]);
        Assert.Equal((byte)(payloadLength >> 8), frame[2]);
        Assert.Equal((byte)payloadLength, frame[3]);

        using var stream = new MemoryStream(frame);
        string json = RpcFrame.Read(stream, 16 * 1024 * 1024 + 1024);
        MessageServer game = RawMessage.Parse(json).ToGameMessage<MessageServer>();

        Assert.Equal(2013, (int)game.Method);
        Assert.Equal(text, game.GetParam<string>());
    }

    [Fact]
    public void ShouldRejectMessagesBeyondTheGamesFrameLimit()
    {
        var raw = new RawMessage(2013);
        raw.Add(new string('x', 16_000_000));
        Assert.Throws<InvalidDataException>(() => RpcFrame.Serialize(raw.ToObject<JObject>()));
    }

    [Fact]
    public void ShouldUseTheGamesStringCompressionEnvelope()
    {
        byte[] shortText = CompressString.StringCompressor.Zip("short");
        Assert.Equal(0, shortText[0]);
        Assert.Equal("short", CompressString.StringCompressor.Unzip(shortText));

        string longText = string.Concat(Enumerable.Repeat("a repetitive source line\n", 100));
        byte[] zipped = CompressString.StringCompressor.Zip(longText);
        Assert.Equal(1, zipped[0]);
        Assert.Equal(longText, CompressString.StringCompressor.Unzip(zipped));
    }
}
