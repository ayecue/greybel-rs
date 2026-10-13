using GreyHackUtil.Contracts;
using GreyHackUtil.Protocol;

namespace GreyHackUtil.Tests.Support;

internal static class ProtocolSetup
{
    internal static Direction CreateDirection(params string[] names) =>
        new(
            "Server",
            "IdServer",
            [.. names.Select((name, id) => new Message(Origin.Game, name, id, null, null, []))]
        );
}
