#nullable enable
namespace Duplex;

internal sealed class FrameEncoder
{
    private readonly DuplexWriter writer;
    private long limit = long.MaxValue;

    public FrameEncoder(DuplexWriter writer)
    {
        this.writer = writer;
    }

    public int? LastFrameStart { get; set; }

    public ushort Minor
    {
        set => writer.Minor = value;
    }

    public void Validate(bool validating, long bodyLimit)
    {
        writer.Validate(validating);
        limit = bodyLimit;
    }

    public void WriteMessage(uint typeId, uint? id, Message body)
    {
        var start = StartFrame(FrameKind.Message);
        try
        {
            writer.WriteU32(FrameLayout.GetMessageKey(typeId, id.HasValue));
            if (id.HasValue)
            {
                writer.WriteU32(id.Value);
            }
            var bodyStart = writer.Size;
            body.Write(writer);
            EnforceLimit(bodyStart);
        }
        catch
        {
            writer.Truncate(start);
            throw;
        }
        FinishFrame(start);
    }

    public void WriteEncodedMessage(uint typeId, uint? id, byte[] body)
    {
        var start = StartFrame(FrameKind.Message);
        writer.WriteU32(FrameLayout.GetMessageKey(typeId, id.HasValue));
        if (id.HasValue)
        {
            writer.WriteU32(id.Value);
        }
        writer.WriteRaw(body);
        FinishFrame(start);
    }

    public void WriteRejection(uint id, ErrorCode code, Violation? violation)
    {
        var start = StartFrame(FrameKind.Failure);
        writer.WriteU32(id);
        writer.WriteU32((uint)code);
        violation?.Write(writer);
        FinishFrame(start);
    }

    public void WriteControl(Control control)
    {
        var start = StartFrame(control.Kind);
        try
        {
            WriteControlBody(control);
        }
        catch
        {
            writer.Truncate(start);
            throw;
        }
        FinishFrame(start);
    }

    private void WriteControlBody(Control control)
    {
        switch (control.Kind)
        {
            case FrameKind.Open:
                new Open(
                    control.Id,
                    control.Core.ToVersion(),
                    control.Protocol.ToVersion(),
                    control.Fingerprint
                ).Write(writer);
                writer.WriteRaw(control.Payload);
                break;
            case FrameKind.Ack:
                new Ack(
                    control.Id,
                    control.Connection,
                    control.Core.ToVersion(),
                    control.Protocol.ToVersion(),
                    control.Fingerprint,
                    control.Deprecation
                ).Write(writer);
                writer.WriteRaw(control.Payload);
                break;
            case FrameKind.Refuse:
                new Refuse(
                    control.Id,
                    (uint)control.Reason,
                    control.Core.ToVersion(),
                    control.Protocol.ToVersion(),
                    control.Majors
                ).Write(writer);
                writer.WriteRaw(control.Payload);
                break;
            case FrameKind.Close:
                new Close(control.Reason).Write(writer);
                break;
        }
    }

    private void EnforceLimit(int bodyStart)
    {
        if (writer.Size - bodyStart > limit)
        {
            throw ChannelException.CreateInvalid(new Violation(new uint[0], Requirement.Limit));
        }
    }

    private int StartFrame(FrameKind kind)
    {
        var start = writer.Size;
        writer.WriteByte((byte)kind);
        writer.WriteByte(0);
        return start;
    }

    private void FinishFrame(int start)
    {
        writer.FinishLength(start + 1);
        LastFrameStart = start;
    }
}
