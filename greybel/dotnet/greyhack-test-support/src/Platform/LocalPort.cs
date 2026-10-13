using System.Net;
using System.Net.Sockets;

namespace GreyHackTestSupport.Platform;

/// <summary>Provides loopback TCP port operations.</summary>
internal static class LocalPort
{
    /// <summary>Returns <paramref name="port"/>, selecting an available port when it is 0.</summary>
    public static int Resolve(int port) => port == 0 ? GetAvailable() : port;

    /// <summary>True when <paramref name="port"/> accepts TCP connections.</summary>
    public static bool Listens(int port)
    {
        try
        {
            using var client = new TcpClient();
            client.Connect(IPAddress.Loopback, port);
            return true;
        }
        catch (SocketException)
        {
            return false;
        }
    }

    private static int GetAvailable()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        int port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
    }
}
