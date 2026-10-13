using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Tests.Support;
using NetworkMessages;
using static GreyHackMessageHook.Tests.Support.ConverterSetup;

namespace GreyHackMessageHook.Tests;

public class ConverterTests
{
    [Fact]
    public void ShouldPreserveJsonParameterKinds()
    {
        var nested = new MessageClient(IdClient.SendTerminalWaitingClientRpc, "nested", 42);
        var raw = new RawMessage(15);
        raw.Add(IngenieriaSocial.MailType.Normal);
        raw.Add(new List<string> { "one", "two" });
        raw.Add(nested);
        raw.Add((byte[]?)null);

        MessageServer game = raw.ToGameMessage<MessageServer>();

        Assert.Equal(IngenieriaSocial.MailType.Normal, game.GetParam<IngenieriaSocial.MailType>());
        Assert.Equal(["one", "two"], game.GetParam<List<string>>());
        MessageClient read = game.GetParam<MessageClient>();
        Assert.Equal(IdClient.SendTerminalWaitingClientRpc, read.Method);
        Assert.Equal("nested", read.GetParam<string>());
        Assert.Equal(42, read.GetParam<int>());
        Assert.Null(game.GetParam<byte[]>());
    }

    [Fact]
    public void ShouldKeepNullableAndAbsentParametersDistinct()
    {
        var remote = new GreyHackMessageHook.Protocol.UpdateTerminalRemoteClientRpc(
            "user", "ip", "device", false, "folder", false, false, 7, null
        );
        RawMessage remoteRaw = Proxy.Converter.Encode(remote);
        Assert.Equal(9, remoteRaw.ToObject<Newtonsoft.Json.Linq.JObject>()["params"]!.Count());
        var remoteRoundTrip = Assert.IsType<GreyHackMessageHook.Protocol.UpdateTerminalRemoteClientRpc>(
            Proxy.Converter.Decode(remoteRaw, Role.Provider)
        );
        Assert.Null(remoteRoundTrip.MapPosition);

        var mail = new GreyHackMessageHook.Protocol.PlayerRecibeMailClientRpc(
            "from", "subject", new byte[0], null
        );
        RawMessage mailRaw = Proxy.Converter.Encode(mail);
        Assert.Equal(3, mailRaw.ToObject<Newtonsoft.Json.Linq.JObject>()["params"]!.Count());
        Assert.IsType<GreyHackMessageHook.Protocol.PlayerRecibeMailClientRpc>(
            Proxy.Converter.Decode(mailRaw, Role.Provider)
        );

        var folders = new GreyHackMessageHook.Protocol.SendPendingFoldersClientRpc(
            new byte[0], null!, null
        );
        var foldersRoundTrip = Assert.IsType<GreyHackMessageHook.Protocol.SendPendingFoldersClientRpc>(
            Proxy.Converter.Decode(Proxy.Converter.Encode(folders), Role.Provider)
        );
        Assert.Empty(foldersRoundTrip.ZipEndCopy);
        Assert.Null(foldersRoundTrip.TrashHasItems);
    }

    [Theory]
    [MemberData(nameof(GetMessages), MemberType = typeof(ConverterSetup))]
    public void ShouldWriteEveryMessageAsTheGamesReaderReadsIt(string name)
    {
        MessageType type = MessageTypeTable.GetMessageType(name);
        Duplex.Message message = Filler.CreateMessage(type);

        RawMessage raw = Proxy.Converter.Encode(message);
        NetworkMessages.RpcMessage game =
            type.Sender == Role.Client
                ? raw.ToGameMessage<MessageServer>()
                : raw.ToGameMessage<MessageClient>();
        IReadOnlyList<string> read = GameReader.Read(game, type).Select(ValueText.Format).ToList();

        Assert.Equal(MessageTypeTable.GetValues(type, message).Select(ValueText.Format), read);
    }

    [Theory]
    [MemberData(nameof(GetMessages), MemberType = typeof(ConverterSetup))]
    public void ShouldReadEveryMessageBackAsWritten(string name)
    {
        MessageType type = MessageTypeTable.GetMessageType(name);
        Duplex.Message message = Filler.CreateMessage(type);

        Duplex.Message? roundTrip = Proxy.Converter.Decode(
            Proxy.Converter.Encode(message),
            type.Sender
        );

        Assert.NotNull(roundTrip);
        Assert.Equal(
            MessageTypeTable.GetValues(type, message).Select(ValueText.Format),
            MessageTypeTable.GetValues(type, roundTrip).Select(ValueText.Format)
        );
    }
}
