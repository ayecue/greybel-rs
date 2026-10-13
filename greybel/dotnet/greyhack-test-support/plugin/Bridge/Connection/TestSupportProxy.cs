using System;
using System.Collections.Generic;
using System.Net;
using Duplex;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Connection;

/// <summary>
/// Listens for test-only protocol connections. Each connection receives a channel and dedicated request router;
/// replies use the same connection.
/// </summary>
internal sealed class TestSupportProxy : IDisposable
{
    private readonly Hub hub = new Hub();
    private readonly List<TestSupportPluginChannel> channels = new List<TestSupportPluginChannel>();
    private TcpProvider provider;

    /// <summary>Creates the request router for a new connection.</summary>
    private Func<TestConnection, TestSupportPluginRouter> createRouter;

    public IPEndPoint Endpoint =>
        provider?.Endpoint ?? throw new InvalidOperationException("The test proxy isn't started");

    public void Start(
        IPEndPoint endpoint,
        Func<TestConnection, TestSupportPluginRouter> createRouter
    )
    {
        if (provider != null)
        {
            throw new InvalidOperationException("The test proxy is already started");
        }
        this.createRouter = createRouter;
        provider = new TcpProvider(hub, endpoint, Accept);
        hub.Start("greyhack test support hub");
        provider.Start();
    }

    private void Accept(StreamDriver driver)
    {
        var connection = new TestConnection(hub);
        var channel = new TestSupportPluginChannel(driver.Transport, createRouter(connection));
        connection.Attach(channel);
        channels.Add(channel);
        channel.OnClose += _ => channels.Remove(channel);
    }

    public void Dispose()
    {
        if (provider == null)
        {
            return;
        }
        provider.Dispose();
        hub.Post(() =>
        {
            foreach (TestSupportPluginChannel channel in channels.ToArray())
            {
                channel.Close();
            }
            hub.Stop();
        });
        provider = null;
    }
}
