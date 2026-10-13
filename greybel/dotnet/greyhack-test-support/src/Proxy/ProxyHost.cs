using System.Net;
using System.Text;
using Duplex;
using GreyHackMessageHook;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Proxy;

internal static class ProxyHost
{
    public static int Run(int port)
    {
        var output = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false))
        {
            AutoFlush = true,
            NewLine = "\n",
        };
        var converter = new Converter();
        var game = new StandardStreamGame(output, converter);
        using var proxy = new MessageProxy(game, new StandardErrorLog());
        proxy.Start(new IPEndPoint(IPAddress.Loopback, port));
        game.WriteLine($"{{\"port\":{proxy.Endpoint.Port}}}");

        using var input = new StreamReader(Console.OpenStandardInput(), new UTF8Encoding(false));
        string? line;
        while ((line = input.ReadLine()) != null)
        {
            if (line.Trim().Length == 0)
            {
                continue;
            }
            try
            {
                if (converter.Decode(RawMessage.Parse(line), Role.Provider) is Message message)
                {
                    proxy.Publish(message);
                }
            }
            catch (Exception error)
            {
                Console.Error.WriteLine(
                    $"proxy: a line of standard input isn't a message: {error.Message}"
                );
            }
        }
        return 0;
    }
}
