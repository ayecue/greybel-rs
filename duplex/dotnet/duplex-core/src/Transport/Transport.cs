#nullable enable
using System;
using System.Collections.Generic;

namespace Duplex;

public interface IDriver
{
    string Name { get; }

    void Send(byte[] batch);

    byte[] GetRandomBytes();
}

internal readonly struct InboundSlice
{
    public InboundSlice(byte[] batch, Slice slice, uint connection, byte core)
    {
        Batch = batch;
        Slice = slice;
        Connection = connection;
        Core = core;
    }

    public byte[] Batch { get; }

    public Slice Slice { get; }

    public uint Connection { get; }

    public byte Core { get; }
}

internal interface IEndpoint
{
    void Receive(InboundSlice slice);

    void OnDisconnect();

    void Close(ErrorCode reason);
}

public sealed class Transport
{
    private readonly Dictionary<ulong, IEndpoint> endpoints = new Dictionary<ulong, IEndpoint>();
    private readonly InboundAliasTable aliases = new InboundAliasTable();
    private readonly Egress egress = new Egress();
    private readonly Queue<Action> work = new Queue<Action>();

    public Transport(Hub hub, IDriver driver)
    {
        Hub = hub;
        Driver = driver;
    }

    public Hub Hub { get; }

    public IDriver Driver { get; }

    public ErrorCode? EndReason { get; private set; }

    internal bool IsIdle => egress.IsEmpty;

    public void Receive(byte[] batch)
    {
        if (EndReason != null)
        {
            return;
        }
        var reader = BatchReader.FromBytes(batch);
        if (reader != null)
        {
            RouteBatch(batch, reader);
        }
        while (work.Count > 0)
        {
            work.Dequeue()();
        }
    }

    private void RouteBatch(byte[] batch, BatchReader reader)
    {
        for (; ; )
        {
            Slice? slice;
            try
            {
                slice = reader.Next();
            }
            catch (MalformedException)
            {
                return;
            }
            if (slice == null)
            {
                return;
            }
            var route = aliases.GetRoute(slice.Value.Key);
            if (route == null)
            {
                continue;
            }
            var local = route.Value.From == Role.Client ? Role.Provider : Role.Client;
            if (
                endpoints.TryGetValue(
                    (ulong)route.Value.Channel * 2 + (ulong)local,
                    out var endpoint
                )
            )
            {
                endpoint.Receive(
                    new InboundSlice(batch, slice.Value, route.Value.Connection, reader.Core)
                );
            }
        }
    }

    public void Disconnect() => End(ErrorCode.ConnectionLoss);

    public void CloseAll(ErrorCode reason)
    {
        foreach (var endpoint in new List<IEndpoint>(endpoints.Values))
        {
            endpoint.Close(reason);
        }
        FlushAll();
    }

    private void End(ErrorCode reason)
    {
        if (EndReason != null)
        {
            return;
        }
        EndReason = reason;
        aliases.Clear();
        egress.Clear();
        work.Clear();
        foreach (var endpoint in new List<IEndpoint>(endpoints.Values))
        {
            if (reason == ErrorCode.ConnectionLoss)
            {
                endpoint.OnDisconnect();
            }
            else
            {
                endpoint.Close(reason);
            }
        }
    }

    internal Action Attach(uint channel, Role role, IEndpoint endpoint)
    {
        var key = (ulong)channel * 2 + (ulong)role;
        if (endpoints.ContainsKey(key))
        {
            throw new InvalidOperationException(
                $"the transport already has a {role} of the channel {channel}"
            );
        }
        endpoints[key] = endpoint;
        return () =>
        {
            if (endpoints.TryGetValue(key, out var current) && current == endpoint)
            {
                endpoints.Remove(key);
                FlushAll();
            }
        };
    }

    internal FrameEncoder Begin(SliceRoute route, ushort minor, long maxFrame) =>
        egress.Begin(route, minor, maxFrame);

    internal void Commit()
    {
        if (!egress.CommitFrame())
        {
            return;
        }
        if (egress.IsFull)
        {
            Flush();
        }
        Hub.OnWrite(this);
    }

    internal void Enqueue(Action task) => work.Enqueue(task);

    internal void Release(SliceRoute route) => egress.Aliases.Release(route);

    internal void Flush()
    {
        if (EndReason != null)
        {
            return;
        }
        egress.Flush(Driver.Send);
    }

    internal void FlushAll()
    {
        while (EndReason == null && !egress.IsEmpty)
        {
            Flush();
        }
    }
}
