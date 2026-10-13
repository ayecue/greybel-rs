using GreyHackMessageHook;

namespace GreyHackTestSupport.Proxy;

internal sealed class StandardErrorLog : IProxyLog
{
    public void LogInfo(string message) => Console.Error.WriteLine(message);

    public void LogError(Exception error) => Console.Error.WriteLine(error);
}
