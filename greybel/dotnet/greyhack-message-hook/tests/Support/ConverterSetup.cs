using Duplex;
using GreyHackMessageHook.Game;
using NetworkMessages;

namespace GreyHackMessageHook.Tests.Support;

internal static class ConverterSetup
{
    public static TheoryData<string> GetMessages() =>
        new(MessageTypeTable.All.Select(type => type.Name));
}
