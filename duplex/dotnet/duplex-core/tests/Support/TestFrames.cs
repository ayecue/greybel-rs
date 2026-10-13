using System.Text;

namespace Duplex.Tests.Support;

internal sealed class MemoryDriver : IDriver
{
    public string Name => "memory";

    public List<byte[]> SentBatches { get; } = new();

    public void Send(byte[] batch) => SentBatches.Add(batch);

    public byte[] GetRandomBytes() => new byte[16];
}

internal sealed record TestBatch(
    uint Channel,
    Role From,
    uint Connection,
    IReadOnlyList<string> Frames
)
{
    public override string ToString() =>
        $"{{ channel {Channel}, from {From}, connection {Connection}, frames [{string.Join(", ", Frames)}] }}";
}

internal static class TestFrames
{
    public static string ToHex(byte[] bytes) => Convert.ToHexString(bytes).ToLowerInvariant();

    public static byte[] Unhex(string text) => Convert.FromHexString(text);

    public sealed class Reader
    {
        private readonly InboundAliasTable aliases = new();

        public List<TestBatch> Read(byte[] postedBatch)
        {
            var batch =
                BatchReader.FromBytes(postedBatch)
                ?? throw new InvalidOperationException("no batch");
            var slices = new List<TestBatch>();
            for (var slice = batch.Next(); slice != null; slice = batch.Next())
            {
                var route =
                    aliases.GetRoute(slice.Value.Key)
                    ?? throw new InvalidOperationException("an alias never announced");
                var frames = new List<string>();
                var reader = new FrameReader(postedBatch, slice.Value.Start, slice.Value.End);
                for (var frame = reader.Next(); frame != null; frame = reader.Next())
                {
                    frames.Add(Describe(frame));
                    reader.SkipFrame();
                }
                slices.Add(new TestBatch(route.Channel, route.From, route.Connection, frames));
            }
            return slices;
        }
    }

    public static TestBatch Decode(byte[] batch)
    {
        var slices = new Reader().Read(batch);
        if (slices.Count != 1)
        {
            throw new InvalidOperationException($"a batch of {slices.Count} slices");
        }
        return slices[0];
    }

    private static string FormatRemainder(DuplexReader body) =>
        ToHex(body.GetRemainder().TakeAll());

    private static string Describe(Frame frame)
    {
        if (frame.Control is { } control)
        {
            return control.Kind switch
            {
                FrameKind.Open =>
                    $"Open(id {control.Id}, core {control.Core}, protocol {control.Protocol}, fingerprint {control.Fingerprint}, payload {ToHex(control.Payload)})",
                FrameKind.Ack =>
                    $"Ack(id {control.Id}, connection {control.Connection}, core {control.Core}, protocol {control.Protocol}, fingerprint {control.Fingerprint}, deprecation {control.Deprecation ?? "null"}, payload {ToHex(control.Payload)})",
                FrameKind.Refuse =>
                    $"Refuse(id {control.Id}, reason {control.Reason}, core {control.Core}, protocol {control.Protocol}, majors [{string.Join(", ", control.Majors)}], payload {ToHex(control.Payload)})",
                FrameKind.Close => $"Close({control.Reason})",
                _ => control.Kind.ToString(),
            };
        }
        return frame.Kind switch
        {
            FrameKind.Message =>
                $"Message(type {frame.TypeId}, id {frame.Id?.ToString() ?? "none"}, body {FormatRemainder(frame.Body!)})",
            FrameKind.Failure when frame.Code == null =>
                $"Failure(id {frame.Id}, declared {FormatRemainder(frame.Body!)})",
            FrameKind.Failure =>
                $"Failure(id {frame.Id}, {frame.Code}, {FormatViolation(frame.Violation)}{FormatRemainder(frame.Body!)})",
            FrameKind.ResponseCredit or FrameKind.MessageCredit =>
                $"{frame.Kind}(id {frame.Id}, count {frame.Count})",
            FrameKind.Cancel => $"Cancel(id {frame.Id})",
            FrameKind.Fragment => $"Fragment(ends {frame.Ends}, bytes {ToHex(frame.Bytes!)})",
            _ => $"{frame.Kind}(id {frame.Id}, body {FormatRemainder(frame.Body!)})",
        };
    }

    private static string FormatViolation(Violation? violation)
    {
        if (violation == null)
        {
            return "";
        }
        var writer = new DuplexWriter();
        violation.Write(writer);
        return ToHex(writer.ToArray());
    }

    public static void WriteRawFrame(
        DuplexWriter writer,
        FrameEncoder encoder,
        uint kind,
        byte[] payload
    )
    {
        var start = writer.Size;
        writer.WriteByte((byte)kind);
        writer.WriteLength(payload.Length);
        writer.WriteRaw(payload);
        encoder.LastFrameStart = start;
    }

    public static byte[] Encode(
        uint channel,
        Role from,
        uint connection,
        Action<DuplexWriter, FrameEncoder> write
    )
    {
        var frames = new DuplexWriter();
        var encoder = new FrameEncoder(frames);
        write(frames, encoder);
        var last =
            encoder.LastFrameStart ?? throw new InvalidOperationException("a batch of no frames");
        var output = new DuplexWriter();
        new BatchWriter(output, BatchWriter.CoreMajor, 1).WriteSlice(
            SliceKey.FromRoute(new SliceRoute(from, channel, connection)),
            frames.Buffer,
            0,
            frames.Size,
            last
        );
        return output.ToArray();
    }

    public static string DecodeUtf8(byte[] bytes) => Encoding.UTF8.GetString(bytes);
}
