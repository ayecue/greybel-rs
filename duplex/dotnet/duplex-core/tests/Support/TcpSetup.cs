using System.Collections.Concurrent;
using System.Net;
using System.Net.Sockets;
using Duplex.Tests.Calculator;

namespace Duplex.Tests.Support;

internal static class TcpSetup
{
    internal static readonly TimeSpan WaitLimit = TimeSpan.FromSeconds(10);

    internal static byte[] ReadBatch(NetworkStream stream)
    {
        var splitter = new BatchStream();
        var receivedBatches = new List<byte[]>();
        var buffer = new byte[1];
        while (receivedBatches.Count == 0)
        {
            var count = stream.Read(buffer, 0, 1);
            if (count == 0)
            {
                throw new EndOfStreamException();
            }
            splitter.Process(buffer, count, receivedBatches.Add);
        }
        return receivedBatches[0];
    }

    internal static void WriteBatch(NetworkStream stream, byte[] batch)
    {
        var prefix = BatchStream.EncodeLengthPrefix(batch.Length);
        stream.Write(prefix);
        stream.Write(batch);
    }

    internal sealed class Logger(BlockingCollection<string> logTexts) : CalculatorRouter
    {
        public override void OnLog(Log message) => logTexts.Add(message.Text);
    }
}
