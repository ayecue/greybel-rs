using Duplex;
using GreyHackMessageHook;
using GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Proxy;

internal sealed class StandardStreamGame(TextWriter output, Converter converter) : IGame
{
    private readonly object gate = new();

    public void Send(Message message) => WriteLine(converter.Encode(message).ToJson());

    public void WriteLine(string line)
    {
        lock (gate)
        {
            output.WriteLine(line);
        }
    }
}
