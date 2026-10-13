using System;
using System.Collections.Generic;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Reads and writes router kernels (<c>/lib/kernel_router.so</c>) on the player's network.</summary>
internal sealed class RouterKernel(Player player)
{
    public const string KernelPath = "/lib/kernel_router.so";

    /// <summary>Gets kernel content from the router at <paramref name="localIp"/>.</summary>
    public string Read(string localIp)
    {
        Router router = player.GetRouterInstances(localIp)[0];

        return (
            router.GetFileSystem().GetArchivo(KernelPath)
            ?? throw new InvalidOperationException("Kernel not found: " + router.GetLocalIP())
        ).GetContenido();
    }

    /// <summary>
    /// Assigns <paramref name="content"/> and its version to every instance of the router at
    /// <paramref name="localIp"/> on the player's network, then saves both values.
    /// </summary>
    public void Write(string localIp, string content)
    {
        string hash = Computer.Md5Sum(content);

        foreach (Router router in player.GetRouterInstances(localIp))
        {
            router.SetAllowSyncDB(true);
            // Create a new library file using VersionsControl.UpgradeLibrary behavior. Replacing both the serialized
            // file ID and cached content ensures Database.RestoreRouter returns the same kernel to later net_use calls.
            router
                .GetFileSystem()
                .ForceAddFile(
                    new FileSystem.Archivo(
                        "kernel_router.so",
                        content,
                        "root",
                        FileSystem.Fichero.TypeFile.Library,
                        saveToDB: true,
                        isDefaultContent: true
                    ),
                    "/lib"
                );
            Database.Singleton.SyncFileSystemDB(router, onlyFileSystem: true);
            if (router.routerNodo != null)
            {
                (
                    (Dictionary<string, string>)
                        GameField.RouterLibraryVersions.GetValue(
                            router.routerNodo.helperLibVersions
                        )
                )["kernel_router.so"] = hash;
                Database.Singleton.SyncServerMap(router.routerNodo);
            }
        }
    }
}
