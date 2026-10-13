using Duplex;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Connection;

/// <summary>Test-to-bridge connection used to return request replies.</summary>
internal sealed class TestConnection(Hub hub)
{
    private TestSupportPluginChannel channel;
    private bool closed;

    /// <summary>Starts sending through <paramref name="connectionChannel"/>, created with this connection's router.</summary>
    public void Attach(TestSupportPluginChannel connectionChannel)
    {
        channel = connectionChannel;
        channel.OnClose += _ => closed = true;
    }

    /// <summary>Sends <paramref name="reply"/> from any thread and discards it if the test has disconnected.</summary>
    public void Send(ReplyRpc reply)
    {
        hub.Post(() =>
        {
            if (!closed)
            {
                channel.Send(reply);
            }
        });
    }
}
