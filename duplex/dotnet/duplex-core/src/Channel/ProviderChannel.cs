#nullable enable
using System;
using System.Collections.Generic;

namespace Duplex;

public enum Health
{
    Healthy,
    Unhealthy,
}

public sealed class ActiveConnection
{
    public ActiveConnection(
        uint connection,
        VersionNumber version,
        VersionNumber? deprecatedVersion,
        string? deprecation
    )
    {
        Connection = connection;
        Version = version;
        DeprecatedVersion = deprecatedVersion;
        DeprecationNote = deprecation;
    }

    public uint Connection { get; }

    public VersionNumber Version { get; }

    public VersionNumber? DeprecatedVersion { get; }

    public string? DeprecationNote { get; }
}

public abstract class ProviderChannel
{
    private static readonly VersionNumber CoreVersion = new VersionNumber(1, 0);
    private const uint ConnectionMultiplier = 0x93d765dd;
    private const int ConnectionRotation = 15;

    private readonly Transport transport;
    private readonly ProtocolDefinition protocol;
    private readonly Router router;
    private readonly ChannelSettings settings;
    private readonly Action detach;
    private readonly Queue<(MessageType Type, byte[] Body)> queue =
        new Queue<(MessageType, byte[])>();
    private readonly HashSet<uint> pendingExchangeIds = new HashSet<uint>();
    private IDisposable? healthCheckTimer;
    private Active? active;
    private Control? ack;
    private uint missedChecks;
    private bool activitySinceCheck;
    private Health reportedHealth = Health.Healthy;
    private Reassembly? reassembly;

    protected ProviderChannel(
        Transport transport,
        ProtocolDefinition protocol,
        Router router,
        ChannelOptions? options
    )
    {
        if (protocol.Core.Major != CoreVersion.Major || protocol.Core.Minor > CoreVersion.Minor)
        {
            throw new ArgumentException(
                $"\"{protocol.Channel}\" builds on the core {protocol.Core}, which this core doesn't speak"
            );
        }
        this.transport = transport;
        this.protocol = protocol;
        this.router = router;
        protocol.Flavours.TryGetValue(transport.Driver.Name, out var flavour);
        settings = new ChannelSettings(options ?? new ChannelOptions(), flavour);
        detach = transport.Attach(protocol.ChannelId, Role.Provider, new Endpoint(this));
        transport.Hub.Post(Start);
    }

    public uint? Connection => active?.Connection;

    public VersionNumber? Version => active?.Protocol;

    public ErrorCode? CloseReason { get; private set; }

    public Health Health =>
        active != null
        && settings.HealthCheck != null
        && missedChecks > settings.HealthCheck.UnhealthyThreshold
            ? Health.Unhealthy
            : Health.Healthy;

    public event Action<ActiveConnection>? OnAck;

    public event Action<ErrorCode>? OnEnd;

    public event Action<ErrorCode>? OnClose;

    public event Action<Health>? OnHealth;

    public void Close(ErrorCode reason = ErrorCode.ClosedChannel) => Close(reason, true);

    public void RunHealthCheck()
    {
        var healthCheck = settings.HealthCheck;
        var current = active;
        if (healthCheck == null || current == null || CloseReason != null)
        {
            return;
        }
        if (activitySinceCheck)
        {
            activitySinceCheck = false;
            return;
        }
        missedChecks++;
        if (missedChecks > healthCheck.CloseThreshold)
        {
            EndAndListen(ErrorCode.Timeout, true);
        }
        else
        {
            WriteControl(new Control(FrameKind.Ping), current.Connection);
        }
        ReportHealth();
    }

    private void RecordActivity()
    {
        activitySinceCheck = true;
        if (missedChecks == 0)
        {
            return;
        }
        missedChecks = 0;
        ReportHealth();
    }

    public void Send(Message message)
    {
        var type = message.GetMessageType();
        var inbound = protocol.Messages[(int)Role.Client];
        if (type.Id >= inbound.Length || inbound[type.Id] != type || type.Responds || type.Streams)
        {
            throw new ArgumentException(
                $"{type} isn't a notification the client of \"{protocol.Channel}\" handles",
                nameof(message)
            );
        }
        if (CloseReason != null)
        {
            throw ChannelException.CreateChannelEnd(CloseReason.Value);
        }
        if (active == null)
        {
            if (queue.Count >= settings.MaxQueue)
            {
                throw ChannelException.CreateRejection(ErrorCode.Overload);
            }
            var writer = new DuplexWriter();
            writer.Validate(settings.Validating);
            message.Write(writer);
            queue.Enqueue((type, writer.ToArray()));
            return;
        }
        var frames = transport.Begin(
            GetRoute(active.Connection),
            active.Protocol.Minor,
            settings.MaxMessage
        );
        try
        {
            frames.Validate(settings.Validating, long.MaxValue);
            if (type.Since > active.Protocol.Minor)
            {
                throw ChannelException.CreateUnavailable();
            }
            frames.WriteMessage(type.Id, null, message);
        }
        finally
        {
            transport.Commit();
        }
    }

    private void Start()
    {
        if (CloseReason != null)
        {
            return;
        }
        var endReason = protocol.Security != null ? ErrorCode.Insecure : transport.EndReason;
        if (endReason != null)
        {
            Close(endReason.Value, false);
            return;
        }
        if (settings.HealthCheck != null)
        {
            healthCheckTimer = transport.Hub.ScheduleInterval(
                settings.HealthCheck.Interval,
                RunHealthCheck
            );
        }
        Listen();
    }

    private void Listen()
    {
        missedChecks = 0;
        activitySinceCheck = false;
        WriteControl(new Control(FrameKind.Listen), 0);
    }

    private void Close(ErrorCode reason, bool notifyRemote)
    {
        if (CloseReason != null)
        {
            return;
        }
        var connection = active?.Connection;
        active = null;
        ack = null;
        CloseReason = reason;
        if (notifyRemote && connection != null)
        {
            WriteControl(new Control(FrameKind.Close) { Reason = reason }, connection.Value);
        }
        EndConnection(connection, reason);
        transport.FlushAll();
        healthCheckTimer?.Dispose();
        healthCheckTimer = null;
        detach();
        OnClose?.Invoke(reason);
    }

    private void EndAndListen(ErrorCode reason, bool notifyRemote)
    {
        var connection = active!.Connection;
        active = null;
        ack = null;
        if (notifyRemote)
        {
            WriteControl(new Control(FrameKind.Close) { Reason = reason }, connection);
        }
        EndConnection(connection, reason);
        Listen();
    }

    private void EndConnection(uint? connection, ErrorCode reason)
    {
        if (connection != null)
        {
            transport.Release(GetRoute(connection.Value));
            OnEnd?.Invoke(reason);
        }
        reassembly = null;
        pendingExchangeIds.Clear();
    }

    private void ReportHealth()
    {
        var health = Health;
        if (health == reportedHealth)
        {
            return;
        }
        reportedHealth = health;
        if (active != null)
        {
            OnHealth?.Invoke(health);
        }
    }

    private void OnControl(Control control, uint connection)
    {
        if (CloseReason != null)
        {
            return;
        }
        var current = active;
        switch (control.Kind)
        {
            case FrameKind.Open:
                Negotiate(control, current?.OpenId);
                break;
            case FrameKind.Ping when current != null && connection == current.Connection:
                RecordActivity();
                WriteControl(new Control(FrameKind.Pong), connection);
                break;
            case FrameKind.Pong when current != null && connection == current.Connection:
                RecordActivity();
                break;
            case FrameKind.Close when current != null && connection == current.Connection:
                EndAndListen(control.Reason, false);
                break;
        }
        ReportHealth();
    }

    private void Negotiate(Control open, uint? openId)
    {
        var version = protocol.Version;
        if (open.Protocol.Major != version.Major || open.Core.Major != CoreVersion.Major)
        {
            Refuse(open.Id, ErrorCode.VersionMismatch);
            return;
        }
        var info = protocol.GetVersion(open.Protocol.Minor);
        if (open.Protocol.Minor <= version.Minor && info?.Fingerprint != open.Fingerprint)
        {
            Refuse(open.Id, ErrorCode.VersionMismatch);
            return;
        }
        if (info?.Unsupported == true)
        {
            Refuse(open.Id, ErrorCode.UnsupportedVersion);
            return;
        }
        if (openId != null)
        {
            if (openId != open.Id)
            {
                Refuse(open.Id, ErrorCode.Busy);
            }
            else if (ack != null)
            {
                WriteControl(ack, 0);
            }
            return;
        }
        var agreed = open.Protocol.WithLowerMinor(version);
        var core = CoreVersion.WithLowerMinor(open.Core);
        var connection = GetConnectionId(transport.Driver.GetRandomBytes());
        ack = new Control(FrameKind.Ack)
        {
            Id = open.Id,
            Connection = connection,
            Core = core,
            Protocol = agreed,
            Fingerprint = protocol.GetVersion(agreed.Minor)?.Fingerprint ?? 0,
            Deprecation = info?.DeprecationNote,
        };
        missedChecks = 0;
        activitySinceCheck = false;
        active = new Active(connection, open.Id, core, agreed);
        WriteControl(ack, 0);
        SendQueue();
        OnAck?.Invoke(
            new ActiveConnection(
                connection,
                agreed,
                info?.DeprecationNote == null ? null : open.Protocol,
                info?.DeprecationNote
            )
        );
    }

    private void Refuse(uint id, ErrorCode reason)
    {
        WriteControl(
            new Control(FrameKind.Refuse)
            {
                Id = id,
                Reason = reason,
                Core = CoreVersion,
                Protocol = protocol.Version,
                Majors = new[] { protocol.Version.Major },
            },
            0
        );
    }

    private void SendQueue()
    {
        var current = active!;
        while (queue.Count > 0)
        {
            var (type, body) = queue.Dequeue();
            var frames = transport.Begin(
                GetRoute(current.Connection),
                current.Protocol.Minor,
                settings.MaxMessage
            );
            try
            {
                if (current.Protocol.Minor >= protocol.Version.Minor)
                {
                    frames.WriteEncodedMessage(type.Id, null, body);
                }
                else if (type.Since <= current.Protocol.Minor)
                {
                    frames.WriteMessage(type.Id, null, type.Read(new DuplexReader(body)));
                }
            }
            catch (Exception error) when (error is ChannelException || error is MalformedException)
            { }
            finally
            {
                transport.Commit();
            }
        }
    }

    private static uint GetConnectionId(byte[] random)
    {
        uint hash = 0;
        for (var offset = 0; offset + 4 <= random.Length; offset += 4)
        {
            var word = (uint)(
                random[offset]
                | (random[offset + 1] << 8)
                | (random[offset + 2] << 16)
                | (random[offset + 3] << 24)
            );
            hash = unchecked((hash + word) * ConnectionMultiplier);
        }
        var id = (hash << ConnectionRotation) | (hash >> (32 - ConnectionRotation));
        return id == 0 ? 1 : id;
    }

    private SliceRoute GetRoute(uint connection) =>
        new SliceRoute(Role.Provider, protocol.ChannelId, connection);

    private void WriteControl(Control control, uint connection)
    {
        var frames = transport.Begin(
            GetRoute(connection),
            active?.Core.Minor ?? DuplexWriter.Newest,
            long.MaxValue
        );
        try
        {
            frames.WriteControl(control);
        }
        finally
        {
            transport.Commit();
        }
    }

    private void WriteRejection(
        uint connection,
        uint exchangeId,
        ErrorCode code,
        Violation? violation = null
    )
    {
        var current = active;
        if (current == null || current.Connection != connection)
        {
            return;
        }
        try
        {
            var frames = transport.Begin(
                GetRoute(connection),
                current.Protocol.Minor,
                settings.MaxMessage
            );
            try
            {
                frames.WriteRejection(exchangeId, code, violation);
            }
            finally
            {
                transport.Commit();
            }
        }
        catch (ChannelException) { }
    }

    private void ReadFrames(byte[] batch, int start, int end, uint connection, byte core)
    {
        var reader = new FrameReader(batch, start, end);
        try
        {
            for (; ; )
            {
                reader.CoreMinor = active?.Core.Minor ?? DuplexWriter.Newest;
                reader.ProtocolMinor = active?.Protocol.Minor ?? DuplexWriter.Newest;
                var frame = reader.Next();
                if (frame == null)
                {
                    return;
                }
                if (frame.Kind == FrameKind.Fragment)
                {
                    OnFragment(connection, core, frame.Bytes!, frame.Ends);
                }
                else if (
                    reader.LastFrameLength > settings.MaxMessage
                    && !FrameLayout.IsControl(frame.Kind)
                )
                {
                    var bytes = new byte[reader.LastFrameLength];
                    Array.Copy(batch, reader.LastFrameStart, bytes, 0, bytes.Length);
                    RefuseOverLimit(connection, FrameReader.ReadHead(bytes));
                }
                else if (core == CoreVersion.Major || FrameLayout.IsFrozen(frame.Kind))
                {
                    OnFrame(connection, frame);
                }
                reader.SkipFrame();
            }
        }
        catch (Exception error)
            when (error is MalformedException
                || error is ChannelException
                || error is ArgumentException
            ) { }
    }

    private void OnFrame(uint connection, Frame frame)
    {
        if (frame.Control != null)
        {
            OnControl(frame.Control, connection);
            return;
        }
        var current = active;
        if (CloseReason != null || current == null || connection != current.Connection)
        {
            return;
        }
        RecordActivity();
        if (frame.Kind != FrameKind.Message)
        {
            return;
        }
        var types = protocol.Messages[(int)Role.Provider];
        var type = frame.TypeId < types.Length ? types[frame.TypeId] : null;
        if (frame.Id == null)
        {
            if (type != null)
            {
                var body = frame.Body!.GetRemainder();
                transport.Enqueue(() => HandleNotification(connection, type, body));
            }
            return;
        }
        var exchangeId = frame.Id.Value;
        if (pendingExchangeIds.Contains(exchangeId))
        {
            return;
        }
        if (type == null || !type.Responds)
        {
            WriteRejection(connection, exchangeId, ErrorCode.UnhandledMessage);
            return;
        }
        pendingExchangeIds.Add(exchangeId);
        var request = frame.Body!.GetRemainder();
        transport.Enqueue(() => RejectRequest(connection, exchangeId, type, request));
    }

    private void HandleNotification(uint connection, MessageType type, DuplexReader body)
    {
        var current = active;
        if (
            current == null
            || current.Connection != connection
            || type.Since > current.Protocol.Minor
        )
        {
            return;
        }
        Message message;
        try
        {
            message = ReadBody(type, body);
        }
        catch (Exception error)
            when (error is MalformedException
                || error is ChannelException
                || error is ArgumentException
            )
        {
            return;
        }
        router.Notify(message);
    }

    private void RejectRequest(
        uint connection,
        uint exchangeId,
        MessageType type,
        DuplexReader body
    )
    {
        if (!pendingExchangeIds.Remove(exchangeId))
        {
            return;
        }
        var current = active;
        if (current == null || current.Connection != connection)
        {
            return;
        }
        if (type.Since > current.Protocol.Minor)
        {
            WriteRejection(connection, exchangeId, ErrorCode.UnsupportedVersion);
            return;
        }
        try
        {
            ReadBody(type, body);
        }
        catch (ChannelException error)
        {
            WriteRejection(connection, exchangeId, error.Code, error.Violation);
            return;
        }
        catch (Exception error) when (error is MalformedException || error is ArgumentException)
        {
            WriteRejection(connection, exchangeId, ErrorCode.MalformedMessage);
            return;
        }
        WriteRejection(connection, exchangeId, ErrorCode.UnhandledMessage);
    }

    private static Message ReadBody(MessageType type, DuplexReader body)
    {
        if (body.UnreadLength > type.Limit)
        {
            throw FieldValidation.CreateViolation(null, Requirement.Limit);
        }
        body.Validating = true;
        var message = type.Read(body);
        body.Finish();
        return message;
    }

    private void OnFragment(uint connection, byte core, byte[] bytes, bool ends)
    {
        var fragments = reassembly;
        if (fragments == null)
        {
            var head = FrameReader.ReadHead(bytes);
            var oversized =
                head == null
                || (
                    head.Value.Length > settings.MaxMessage
                    && !FrameLayout.IsControl(head.Value.Kind)
                );
            fragments = reassembly = new Reassembly(oversized ? 0 : head!.Value.Length);
            if (oversized)
            {
                RefuseOverLimit(connection, head);
            }
        }
        if (!fragments.Append(bytes))
        {
            if (ends)
            {
                reassembly = null;
            }
            return;
        }
        if (!ends)
        {
            return;
        }
        reassembly = null;
        ReadFrames(fragments.Bytes, 0, fragments.Bytes.Length, connection, core);
    }

    private void RefuseOverLimit(uint connection, FrameHead? head)
    {
        if (head?.Id != null && head.Value.Kind == FrameKind.Message)
        {
            WriteRejection(connection, head.Value.Id.Value, ErrorCode.Overload);
        }
    }

    private sealed class Active
    {
        public Active(uint connection, uint openId, VersionNumber core, VersionNumber protocol)
        {
            Connection = connection;
            OpenId = openId;
            Core = core;
            Protocol = protocol;
        }

        public uint Connection { get; }

        public uint OpenId { get; }

        public VersionNumber Core { get; }

        public VersionNumber Protocol { get; }
    }

    private sealed class Reassembly
    {
        private int length;
        private bool dropping;

        public Reassembly(int size)
        {
            Bytes = new byte[size];
            dropping = size == 0;
        }

        public byte[] Bytes { get; }

        public bool Append(byte[] piece)
        {
            if (dropping)
            {
                return false;
            }
            if (length + piece.Length > Bytes.Length)
            {
                dropping = true;
                return false;
            }
            Array.Copy(piece, 0, Bytes, length, piece.Length);
            length += piece.Length;
            return true;
        }
    }

    private sealed class Endpoint : IEndpoint
    {
        private readonly ProviderChannel channel;

        public Endpoint(ProviderChannel channel)
        {
            this.channel = channel;
        }

        public void Receive(InboundSlice slice) =>
            channel.ReadFrames(
                slice.Batch,
                slice.Slice.Start,
                slice.Slice.End,
                slice.Connection,
                slice.Core
            );

        public void OnDisconnect() => channel.Close(ErrorCode.ConnectionLoss, false);

        public void Close(ErrorCode reason) => channel.Close(reason);
    }
}
