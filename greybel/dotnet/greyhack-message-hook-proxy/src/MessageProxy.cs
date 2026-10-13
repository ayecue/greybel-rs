#nullable enable
using System;
using System.Collections.Generic;
using System.Net;
using Duplex;
using GreyHackMessageHook.Protocol;

namespace GreyHackMessageHook;

public sealed class MessageProxy : IDisposable
{
    private readonly IGame game;
    private readonly IProxyLog log;
    private readonly object gate = new object();
    private readonly Hub hub = new Hub();
    private readonly List<MessageHookChannel> channels = new List<MessageHookChannel>();
    private TcpProvider? provider;

    public MessageProxy(IGame game, IProxyLog log)
    {
        this.game = game;
        this.log = log;
    }

    public IPEndPoint Endpoint =>
        (provider ?? throw new InvalidOperationException("the proxy isn't started")).Endpoint;

    public void Start(IPEndPoint endpoint)
    {
        lock (gate)
        {
            if (provider != null)
            {
                throw new InvalidOperationException("the proxy is already started");
            }
            provider = new TcpProvider(hub, endpoint, Accept);
            hub.Start("greybel proxy hub");
            provider.Start();
        }
        log.LogInfo("Proxy - listening on " + Endpoint);
    }

    public void Publish(Message message)
    {
        if (provider == null)
        {
            return;
        }
        hub.Post(() =>
        {
            foreach (MessageHookChannel channel in channels)
            {
                try
                {
                    channel.Send(message);
                }
                catch (ChannelException error)
                {
                    log.LogInfo(
                        "Proxy - " + message.GetMessageType().Name + " isn't sent: " + error.Message
                    );
                }
            }
        });
    }

    private void Accept(StreamDriver driver)
    {
        var channel = new MessageHookChannel(driver.Transport, new GameRouter(this));
        channels.Add(channel);
        channel.OnClose += reason =>
        {
            channels.Remove(channel);
            log.LogInfo("Proxy - a channel closed as " + reason);
        };
    }

    private void Receive(Message message)
    {
        try
        {
            game.Send(message);
        }
        catch (Exception error)
        {
            log.LogError(error);
        }
    }

    public void Dispose()
    {
        lock (gate)
        {
            if (provider == null)
            {
                return;
            }
            provider.Dispose();
            hub.Post(() =>
            {
                foreach (MessageHookChannel channel in channels.ToArray())
                {
                    channel.Close();
                }
                hub.Stop();
            });
        }
        log.LogInfo("Proxy - stopped");
    }

    private sealed class GameRouter : MessageHookRouter
    {
        private readonly MessageProxy proxy;

        public GameRouter(MessageProxy proxy)
        {
            this.proxy = proxy;
        }

        public override void Notify(Message message) => proxy.Receive(message);
    }
}
