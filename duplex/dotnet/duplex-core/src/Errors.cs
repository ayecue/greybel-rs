#nullable enable
using System;

namespace Duplex;

public sealed class ChannelException : Exception
{
    private ChannelException(
        ChannelExceptionKind kind,
        ErrorCode code,
        Violation? violation,
        string message
    )
        : base(message)
    {
        Kind = kind;
        Code = code;
        Violation = violation;
    }

    public ChannelExceptionKind Kind { get; }

    public ErrorCode Code { get; }

    public Violation? Violation { get; }

    public static ChannelException CreateRejection(ErrorCode code) =>
        new ChannelException(ChannelExceptionKind.Rejection, code, null, $"rejected as {code}");

    public static ChannelException CreateUnavailable() =>
        new ChannelException(
            ChannelExceptionKind.Unavailable,
            ErrorCode.UnsupportedVersion,
            null,
            "newer than the minor the connection speaks"
        );

    public static ChannelException CreateInvalid(Violation violation) =>
        new ChannelException(
            ChannelExceptionKind.Rejection,
            ErrorCode.Invalid,
            violation,
            $"a field breaks its requirement {violation.Requirement}"
        );

    public static ChannelException CreateChannelEnd(ErrorCode reason) =>
        new ChannelException(
            ChannelExceptionKind.ChannelEnd,
            reason,
            null,
            $"the channel closed as {reason}"
        );
}

public enum ChannelExceptionKind
{
    Rejection,

    Unavailable,

    ChannelEnd,
}

public sealed class MalformedException : Exception
{
    public MalformedException(string message)
        : base(message) { }
}
