using System;
using System.Threading.Tasks;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Patches;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Restarts the player's computer as <c>ComputerIntrinsics.reboot</c> does from a terminal connected to it, so a test
/// can read what the client's start does (the saved Wi-Fi network connected again, the safe-mode terminal) from the
/// scripts that follow.
/// </summary>
internal sealed class RebootFixture(MainThread main, Player player)
{
    /// <summary>Timeout for the client to start the computer again: closing windows, boot animation and desktop.</summary>
    private static readonly TimeSpan RestartTime = TimeSpan.FromSeconds(120);

    /// <summary>
    /// Restarts the computer on the server (<c>PlayerComputer.Reboot</c>) and has the client's terminal restart it
    /// (<c>PlayerClientMethods.SendRebootClientRpc</c> for a terminal: <c>Terminal.RebootSystem</c>), which requests how the
    /// computer starts from the server (<c>PlayerHelperServer.CheckIniErrorServerRpc</c>). Replies once it has started.
    /// </summary>
    public void Restart(Requester requester, RebootPlayerRpc request)
    {
        main.Post(
            requester,
            async () =>
            {
                TaskCompletionSource<bool> completion = new TaskCompletionSource<bool>(
                    TaskCreationOptions.RunContinuationsAsynchronously
                );
                Action onReady = () => completion.TrySetResult(true);

                if (request.SafeMode)
                {
                    RebootPatch.SafeModeReady += onReady;
                }
                else
                {
                    RebootPatch.DesktopReady += onReady;
                }

                try
                {
                    player.Computer.Reboot(request.SafeMode, -1);
                    (
                        InternalBash.Singleton
                        ?? throw new InvalidOperationException("The desktop isn't loaded")
                    ).RebootSystem(false, true, request.SafeMode);

                    if (
                        await Task.WhenAny(completion.Task, Task.Delay(RestartTime))
                        != completion.Task
                    )
                    {
                        throw new InvalidOperationException(
                            "The game didn't start the player's computer again"
                        );
                    }

                    requester.Reply(new Reply.Applied());
                }
                finally
                {
                    RebootPatch.SafeModeReady -= onReady;
                    RebootPatch.DesktopReady -= onReady;
                }
            }
        );
    }
}
