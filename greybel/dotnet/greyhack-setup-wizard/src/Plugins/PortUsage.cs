using System.Net;
using System.Net.Sockets;

namespace GreyHackSetupWizard.Plugins;

public static class PortUsage
{
    public static bool IsInUse(int port)
    {
        var listener = new TcpListener(IPAddress.Loopback, port);
        try
        {
            listener.Start();
            return false;
        }
        catch (SocketException error)
            when (error.SocketErrorCode
                    is SocketError.AddressAlreadyInUse
                        or SocketError.AccessDenied
            )
        {
            return true;
        }
        finally
        {
            listener.Stop();
        }
    }
}
