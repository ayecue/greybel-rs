using System;
using System.Threading;
using System.Threading.Tasks;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Connection;

/// <summary>
/// Executes every request on the game main thread because game APIs are not thread-safe. Request failures return
/// the error and stack to the originating test.
/// </summary>
internal sealed class MainThread(SynchronizationContext context)
{
    /// <summary>Executes <paramref name="operation"/> and sends its return value as the reply.</summary>
    public void Respond(Requester requester, Func<Reply> operation)
    {
        Post(requester, () => requester.Reply(operation()));
    }

    /// <summary>Executes <paramref name="action"/>, which sends its own reply.</summary>
    public void Post(Requester requester, Action action)
    {
        Run(action, requester.Fail);
    }

    /// <summary>Awaits <paramref name="action"/> through any game waits; the action sends its own reply.</summary>
    public void Post(Requester requester, Func<Task> action)
    {
        context.Post(
            async _ =>
            {
                try
                {
                    await action();
                }
                catch (Exception error)
                {
                    requester.Fail(error);
                }
            },
            null
        );
    }

    /// <summary>Executes non-request <paramref name="action"/>, such as game-message handling, and logs failures.</summary>
    public void Post(Action action)
    {
        Run(action, error => UnityEngine.Debug.LogError("GreyHackTestSupport - " + error));
    }

    private void Run(Action action, Action<Exception> fail)
    {
        try
        {
            context.Post(
                _ =>
                {
                    try
                    {
                        action();
                    }
                    catch (Exception error)
                    {
                        fail(error);
                    }
                },
                null
            );
        }
        catch (Exception error)
        {
            fail(error);
        }
    }
}
