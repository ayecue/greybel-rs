#nullable enable
using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Security.Cryptography;
using System.Threading;

namespace Duplex;

public sealed class BatchStream
{
    public const int MaxBatchBytes = 64 * 1024 * 1024;
    private const int MaxLengthBytes = 4;

    private byte[] buffer = new byte[64 * 1024];
    private int start;
    private int end;

    public static byte[] EncodeLengthPrefix(int length)
    {
        var writer = new DuplexWriter(8);
        writer.WriteLength(length);
        return writer.ToArray();
    }

    public void Process(byte[] bytes, int count, Action<byte[]> receive)
    {
        if (end + count > buffer.Length)
        {
            var unread = end - start;
            var size = Math.Max(buffer.Length, unread + count);
            var larger =
                size > buffer.Length ? new byte[Math.Max(size, buffer.Length * 2)] : buffer;
            Array.Copy(buffer, start, larger, 0, unread);
            buffer = larger;
            start = 0;
            end = unread;
        }
        Array.Copy(bytes, 0, buffer, end, count);
        end += count;
        for (; ; )
        {
            var length = 0;
            var index = start;
            var hasLength = false;
            while (index < end && index - start < MaxLengthBytes)
            {
                var current = buffer[index];
                length |= (current & 0x7f) << (7 * (index - start));
                index++;
                if ((current & 0x80) == 0)
                {
                    hasLength = true;
                    break;
                }
            }
            if (!hasLength)
            {
                if (index - start >= MaxLengthBytes)
                {
                    throw new MalformedException(
                        $"a batch's length takes more than {MaxLengthBytes} bytes"
                    );
                }
                break;
            }
            if (length > MaxBatchBytes)
            {
                throw new MalformedException(
                    $"a batch of {length} bytes is longer than {MaxBatchBytes}"
                );
            }
            if (index + length > end)
            {
                break;
            }
            var batch = new byte[length];
            Array.Copy(buffer, index, batch, 0, length);
            start = index + length;
            receive(batch);
        }
        if (start == end)
        {
            start = end = 0;
        }
    }
}

public sealed class StreamDriver : IDriver, IDisposable
{
    private readonly Stream stream;
    private readonly Hub hub;
    private Thread? reader;
    private int alive = 1;

    public StreamDriver(Hub hub, Stream stream, string name)
    {
        this.hub = hub;
        this.stream = stream;
        Name = name;
        Transport = new Transport(hub, this);
    }

    public string Name { get; }

    public Transport Transport { get; }

    public static StreamDriver FromTcpClient(Hub hub, TcpClient client)
    {
        client.NoDelay = true;
        return new StreamDriver(hub, client.GetStream(), "tcp");
    }

    public void Start()
    {
        reader = new Thread(Read) { IsBackground = true, Name = $"duplex {Name} reader" };
        reader.Start();
    }

    public void Send(byte[] batch)
    {
        if (Volatile.Read(ref alive) == 0)
        {
            return;
        }
        try
        {
            var prefix = BatchStream.EncodeLengthPrefix(batch.Length);
            stream.Write(prefix, 0, prefix.Length);
            stream.Write(batch, 0, batch.Length);
            stream.Flush();
        }
        catch (Exception error) when (error is IOException || error is ObjectDisposedException)
        {
            End();
        }
    }

    public byte[] GetRandomBytes()
    {
        var bytes = new byte[16];
        using (var generator = RandomNumberGenerator.Create())
        {
            generator.GetBytes(bytes);
        }
        return bytes;
    }

    private void Read()
    {
        var batches = new BatchStream();
        var bytes = new byte[64 * 1024];
        try
        {
            for (; ; )
            {
                var count = stream.Read(bytes, 0, bytes.Length);
                if (count == 0)
                {
                    break;
                }
                batches.Process(bytes, count, batch => hub.Post(() => Transport.Receive(batch)));
            }
        }
        catch (Exception error)
            when (error is IOException
                || error is ObjectDisposedException
                || error is MalformedException
            ) { }
        End();
    }

    private void End()
    {
        if (Interlocked.Exchange(ref alive, 0) == 0)
        {
            return;
        }
        hub.Post(Transport.Disconnect);
        try
        {
            stream.Dispose();
        }
        catch (IOException) { }
    }

    public void Dispose() => End();
}

public sealed class TcpProvider : IDisposable
{
    private readonly TcpListener listener;
    private readonly Hub hub;
    private readonly Action<StreamDriver> accept;
    private Thread? acceptor;
    private volatile bool stopping;

    public TcpProvider(Hub hub, IPEndPoint endpoint, Action<StreamDriver> accept)
    {
        this.hub = hub;
        this.accept = accept;
        listener = new TcpListener(endpoint);
    }

    public IPEndPoint Endpoint => (IPEndPoint)listener.LocalEndpoint;

    public void Start()
    {
        listener.Start();
        acceptor = new Thread(Accept) { IsBackground = true, Name = "duplex tcp acceptor" };
        acceptor.Start();
    }

    private void Accept()
    {
        while (!stopping)
        {
            TcpClient client;
            try
            {
                client = listener.AcceptTcpClient();
            }
            catch (Exception error)
                when (error is SocketException
                    || error is ObjectDisposedException
                    || error is InvalidOperationException
                )
            {
                return;
            }
            var driver = StreamDriver.FromTcpClient(hub, client);
            hub.Post(() =>
            {
                accept(driver);
                driver.Start();
            });
        }
    }

    public void Dispose()
    {
        stopping = true;
        listener.Stop();
    }
}
