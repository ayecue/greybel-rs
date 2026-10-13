#nullable enable
using System;

namespace GreyHackMessageHook;

public interface IProxyLog
{
    void LogInfo(string message);

    void LogError(Exception error);
}
