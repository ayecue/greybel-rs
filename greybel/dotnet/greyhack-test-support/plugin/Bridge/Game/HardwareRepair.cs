using System.Collections.Generic;
using System.Linq;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Stabilizes game-generated or degraded hardware to prevent random failures during tests.</summary>
internal static class HardwareRepair
{
    /// <summary>
    /// Stabilizes every computer in <paramref name="router"/>'s network before game download and process checks.
    /// Saved health degrades across test runs; below 45, downloads and processes fail randomly. Insufficient power for
    /// generated disks and CPUs causes the same failures. This covers <c>PlayerUtils.IsHardDiskStable</c>,
    /// <c>PlayerUtils.IsHardwareStable</c>, and <c>Hardware.PowerSupply.GetAvailablePower</c>; failed operations block
    /// scripts indefinitely in both message-hook contexts and the game terminal.
    /// </summary>
    public static void RestoreNetwork(Router router)
    {
        NetworkLan lan = router.GetNetworkLan();
        List<Computer> computers = new List<Computer> { router };

        void Collect(NetworkLan.Device device)
        {
            computers.AddRange(
                device.GetLanPcInfos().Select(info => router.GetComputer(info.localIp))
            );

            foreach (NetworkLan.Device child in device.GetDevices())
            {
                Collect(child);
            }
        }

        Collect(lan.routerDevice);

        foreach (Computer computer in computers.Where(computer => computer != null).Distinct())
        {
            Stabilize(computer);
        }
    }

    /// <summary>
    /// Restores all <paramref name="computer"/> hardware to full health and sufficient power (see
    /// <see cref="RestoreNetwork"/>), then saves it. Unsaved generated computers are recreated from their seed with
    /// original hardware on every <c>ServerMap.GetRemoteComputer</c> lookup, so their complete state is persisted with
    /// <c>Database.SyncFileSystemDB</c>.
    /// </summary>
    public static void Stabilize(Computer computer)
    {
        Hardware hardware = computer.GetHardware();

        if (hardware == null)
        {
            return;
        }

        foreach (
            Hardware.ItemHardware component in new Hardware.ItemHardware[]
            {
                hardware.hardDisk,
                hardware.gpu,
                hardware.powerSupply,
                hardware.motherBoard,
            }
                .Concat(hardware.cpus)
                .Concat(hardware.rams)
                .Where(component => component != null)
        )
        {
            component.health = 100f;
        }

        while (hardware.powerSupply.GetAvailablePower(hardware) < 1f)
        {
            hardware.powerSupply.power += 50;
        }

        if (computer.isPlayer)
        {
            Database.Singleton.SyncHardwareDB(computer.GetID(), hardware);
        }
        else
        {
            computer.SetAllowSyncDB(true);
            Database.Singleton.SyncFileSystemDB(computer);
        }
    }
}
