using System.Collections.Generic;
using GreyHackMessageHook.Game;
using HarmonyLib;
using Newtonsoft.Json;

namespace GreyHackMessageHook
{
    /// <summary>
    /// Redirects server messages for a program terminal window to the <see cref="ContextTerminal"/> of the
    /// windowless context running that program. Game handling continues for any other windows named by the
    /// message.
    /// </summary>
    [HarmonyPatch(typeof(PlayerClientMethods))]
    public static class ContextTerminalPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("SendFileSizeClientRpc")]
        static bool SendFileSizeClientRpcPrefix(long size, int multSpeedDisk, byte[] pidRunning)
        {
            List<int> pids = JsonConvert.DeserializeObject<List<int>>(
                StringCompressor.Unzip(pidRunning)
            );

            foreach (int pid in pids)
            {
                FindTerminal(pid)?.ResumeDownload(size, multSpeedDisk, pids.Count);
            }

            return true;
        }

        [HarmonyPrefix]
        [HarmonyPatch("FailedDiskClientRpc")]
        static bool FailedDiskClientRpcPrefix(
            int numProcesos,
            long fileSize,
            int multSpeedDisk,
            int windowPID
        )
        {
            ContextTerminal terminal = FindTerminal(windowPID);

            if (terminal == null)
            {
                return true;
            }

            terminal.ResumeFailedDownload(fileSize, multSpeedDisk, numProcesos);

            return false;
        }

        [HarmonyPrefix]
        [HarmonyPatch("DecipherTimeClientRpc")]
        static bool DecipherTimeClientRpcPrefix(float timeDecipher, byte[] serialTerminals)
        {
            List<int> pids = JsonConvert.DeserializeObject<List<int>>(
                StringCompressor.Unzip(serialTerminals)
            );

            foreach (int pid in pids)
            {
                FindTerminal(pid)?.ResumeDecipher(timeDecipher);
            }

            return true;
        }

        [HarmonyPrefix]
        [HarmonyPatch("FailedProcessClientRpc")]
        static bool FailedProcessClientRpcPrefix(int typeError, float info, int windowPID)
        {
            ContextTerminal terminal = FindTerminal(windowPID);

            if (terminal == null)
            {
                return true;
            }

            terminal.ResumeFailedProcess(info, typeError);

            return false;
        }

        [HarmonyPrefix]
        [HarmonyPatch("RoutersCercanosClientRpc")]
        static bool RoutersCercanosClientRpcPrefix(
            byte[] zipHelperRouters,
            int terminalPID,
            bool isLan,
            string origPublicIp,
            string bssid,
            string essid,
            int maxAcks
        )
        {
            ContextTerminal terminal = FindTerminal(terminalPID);

            if (terminal == null)
            {
                return true;
            }

            if (zipHelperRouters != null && zipHelperRouters.Length != 0)
            {
                List<ServerMap.HelperRoutersCercanos> routers = JsonConvert.DeserializeObject<
                    List<ServerMap.HelperRoutersCercanos>
                >(StringCompressor.Unzip(zipHelperRouters));

                terminal.ResumeAireplay(routers, isLan, origPublicIp, bssid, essid, maxAcks);
            }

            return false;
        }

        [HarmonyPrefix]
        [HarmonyPatch("CdTerminalClientRpc")]
        static bool CdTerminalClientRpcPrefix(string path, int terminalPID)
        {
            ContextTerminal terminal = FindTerminal(terminalPID);

            if (terminal == null)
            {
                return true;
            }

            terminal.SetFolder(path);

            return false;
        }

        [HarmonyPrefix]
        [HarmonyPatch("UpdateTerminalRemoteClientRpc")]
        static bool UpdateTerminalRemoteClientRpcPrefix(
            string activeUser,
            string currentFolder,
            int windowPID
        )
        {
            ContextTerminal terminal = FindTerminal(windowPID);

            if (terminal == null)
            {
                return true;
            }

            terminal.SetConnection(activeUser, currentFolder);

            return false;
        }

        private static ContextTerminal FindTerminal(int pid)
        {
            return ContextManager.GetContextByPid(pid)?.Terminal;
        }
    }
}
