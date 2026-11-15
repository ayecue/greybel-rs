using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Plugin.Bridge.Fixtures;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Patches;
using GreyHackTestSupport.Protocol;
using GameProtocol = GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Setup;

/// <summary>Prepares a new player computer for each test and restores changes from the previous test.</summary>
internal sealed class PlayerSetup(
    MainThread main,
    Player player,
    WorldRestore restore,
    FixtureStore store,
    IReadOnlyList<IFixture> fixtures
)
{
    /// <summary>Timeout for new-player computer creation.</summary>
    private static readonly TimeSpan NewPlayerTime = TimeSpan.FromSeconds(120);

    /// <summary>Serializes setup operations because concurrent setups would wait for the same new computer.</summary>
    private Task currentSetup = Task.CompletedTask;

    /// <summary>
    /// Recreates the player computer through the normal deletion flow. It returns to the main menu with
    /// "delete computer" selected (<c>BiosMenu.UpdateDeletePlayer</c>), logs in with <c>forceDeletePlayer</c>, and lets
    /// the server delete and reinstall the computer (<c>PlayerHelperServer.AllowLogin</c>). Each test receives a new
    /// computer, home, and hardware with the fixture RAM and <c>metaxploit.so</c>. Previous-test world changes are
    /// restored first.
    /// </summary>
    public void ReplaceComputer(Requester requester)
    {
        main.Post(
            requester,
            async () =>
            {
                Task previous = currentSetup;
                TaskCompletionSource<bool> completion = new TaskCompletionSource<bool>();
                currentSetup = completion.Task;

                try
                {
                    await previous;
                    requester.Reply(await CreateComputer());
                }
                finally
                {
                    completion.SetResult(true);
                }
            }
        );
    }

    private async Task<Reply> CreateComputer()
    {
        foreach (IFixture fixture in fixtures)
        {
            fixture.Reset();
        }
        restore.Restore();
        store.Reset();

        TaskCompletionSource<bool> installation = new TaskCompletionSource<bool>();
        Action onInstall = () => installation.TrySetResult(true);

        NewPlayerPatch.NewComputer += onInstall;

        try
        {
            BiosMenuPatch.DeleteComputer = true;
            // Return to the main menu so test support can start single player again.
            PlayerClient.Singleton.Disconnect();

            if (
                await Task.WhenAny(installation.Task, Task.Delay(NewPlayerTime))
                != installation.Task
            )
            {
                throw new InvalidOperationException(
                    "The game didn't create the new player's computer"
                );
            }
        }
        finally
        {
            NewPlayerPatch.NewComputer -= onInstall;
        }

        PlayerServer server = player.Server;
        PlayerComputer computer = server.GetComputer();
        ZeroDayPlayerInfo zeroDay = Database.Singleton.GetZeroDayRequestPlayer(
            server.GetPlayerID()
        );
        zeroDay.ResetLoginAttempts();
        zeroDay.ResetSoloAttempts();
        Database.Singleton.SyncZeroDayRequestPlayer(server.GetPlayerID(), zeroDay, false, false);
        Hardware hardware = computer.GetHardware();
        hardware.rams.Clear();
        hardware.rams.Add(new Hardware.RAM(PlayerFixture.MachineName, 4096, 1, new Random(), true));
        // A new player's CPU makes every decipher and metaxploit scan take about 48 seconds
        // (`PlayerUtils.GetTiempoCracking`: 30 / speed × 1.65); a faster CPU lets tests that run several finish within
        // their time limit.
        foreach (Hardware.CPU cpu in hardware.cpus)
        {
            cpu.speed = PlayerFixture.CpuSpeed;
        }
        // New computers can receive a power supply insufficient for their disks and CPUs.
        HardwareRepair.Stabilize(computer);
        player.User = PlayerFixture.TestUser;
        computer
            .GetFileSystem()
            .ForceAddFile(VersionsControl.Singleton.UpgradeLibrary("metaxploit.so"), "/lib");
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);
        return new Reply.PlayerReady(
            new PlayerReadyReply(
                computer.GetID(),
                player.User,
                computer.GetConfigOS().GetNombreMaquina()
            )
        );
    }
}
