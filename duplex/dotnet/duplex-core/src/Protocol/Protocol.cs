#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;

namespace Duplex;

public abstract class Message
{
    public abstract MessageType GetMessageType();

    public abstract void Write(DuplexWriter writer);
}

public sealed class MessageType
{
    private readonly Func<DuplexReader, Message> read;
    private readonly Func<object?[], Message>? create;

    public MessageType(
        string name,
        uint id,
        ushort since,
        Role sender,
        uint limit,
        bool responds,
        bool streams,
        Func<DuplexReader, Message> read,
        IReadOnlyList<Tag>? tags = null,
        IReadOnlyList<FieldType>? fields = null,
        Func<object?[], Message>? create = null
    )
    {
        Name = name;
        Id = id;
        Since = since;
        Sender = sender;
        Limit = limit;
        Responds = responds;
        Streams = streams;
        this.read = read;
        Tags = tags ?? Array.Empty<Tag>();
        Fields = fields ?? Array.Empty<FieldType>();
        this.create = create;
    }

    public string Name { get; }

    public uint Id { get; }

    public ushort Since { get; }

    public Role Sender { get; }

    public uint Limit { get; }

    public bool Responds { get; }

    public bool Streams { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public IReadOnlyList<FieldType> Fields { get; }

    public Message Read(DuplexReader reader) => read(reader);

    public Message Create(object?[] values) =>
        (create ?? throw new InvalidOperationException($"{Name} is made by its constructor only"))(
            values
        );

    public override string ToString() => Name;
}

public readonly struct VersionNumber : IEquatable<VersionNumber>
{
    public VersionNumber(ushort major, ushort minor)
    {
        Major = major;
        Minor = minor;
    }

    public ushort Major { get; }

    public ushort Minor { get; }

    public static VersionNumber FromVersion(Version version) =>
        new VersionNumber(version.Major, version.Minor);

    public Version ToVersion() => new Version(Major, Minor);

    public VersionNumber WithLowerMinor(VersionNumber other) =>
        new VersionNumber(Major, Math.Min(Minor, other.Minor));

    public bool Equals(VersionNumber other) => Major == other.Major && Minor == other.Minor;

    public override bool Equals(object? other) => other is VersionNumber version && Equals(version);

    public override int GetHashCode() => (Major << 16) | Minor;

    public override string ToString() => $"{Major}.{Minor}";
}

public sealed class VersionInfo
{
    public VersionInfo(uint fingerprint, string? deprecationNote, bool unsupported)
    {
        Fingerprint = fingerprint;
        DeprecationNote = deprecationNote;
        Unsupported = unsupported;
    }

    public uint Fingerprint { get; }

    public string? DeprecationNote { get; }

    public bool Unsupported { get; }
}

public enum SecurityLevel
{
    Encryption,
    Authentication,
}

public sealed class HealthCheck
{
    public static readonly HealthCheck Default = new HealthCheck(5000, 2, 3);

    public HealthCheck(uint interval, uint unhealthyThreshold, uint closeThreshold)
    {
        Interval = interval;
        UnhealthyThreshold = unhealthyThreshold;
        CloseThreshold = closeThreshold;
    }

    public uint Interval { get; }

    public uint UnhealthyThreshold { get; }

    public uint CloseThreshold { get; }

    public HealthCheck With(uint? interval, uint? unhealthyThreshold, uint? closeThreshold) =>
        new HealthCheck(
            interval ?? Interval,
            unhealthyThreshold ?? UnhealthyThreshold,
            closeThreshold ?? CloseThreshold
        );
}

public sealed class Flavour
{
    public Flavour(
        string transport,
        HealthCheck? healthCheck,
        uint? maxMessage,
        bool turnsOffHealthCheck = false
    )
    {
        Transport = transport;
        HealthCheck = healthCheck;
        MaxMessage = maxMessage;
        TurnsOffHealthCheck = turnsOffHealthCheck;
    }

    public string Transport { get; }

    public HealthCheck? HealthCheck { get; }

    public uint? MaxMessage { get; }

    public bool TurnsOffHealthCheck { get; }
}

public sealed class ProtocolDefinition
{
    public ProtocolDefinition(
        string channel,
        uint channelId,
        VersionNumber version,
        VersionNumber core,
        VersionInfo[] versions,
        MessageType[][] messages,
        SecurityLevel? security,
        Flavour[] flavours,
        IReadOnlyList<Tag>? tags = null
    )
    {
        Channel = channel;
        ChannelId = channelId;
        Version = version;
        Core = core;
        Versions = versions;
        Messages = messages;
        Security = security;
        Flavours = flavours.ToDictionary(flavour => flavour.Transport);
        Tags = tags ?? Array.Empty<Tag>();
    }

    public string Channel { get; }

    public uint ChannelId { get; }

    public VersionNumber Version { get; }

    public VersionNumber Core { get; }

    public IReadOnlyList<VersionInfo> Versions { get; }

    public IReadOnlyList<MessageType[]> Messages { get; }

    public SecurityLevel? Security { get; }

    public IReadOnlyDictionary<string, Flavour> Flavours { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public VersionInfo? GetVersion(ushort minor) => minor < Versions.Count ? Versions[minor] : null;
}

public abstract class Router
{
    public abstract void Notify(Message message);
}
