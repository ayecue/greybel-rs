using System;
using System.Reflection;
using System.Threading;
using HarmonyLib;

namespace GreyHackMessageHook.Greybel
{
    [HarmonyPatch(typeof(AppLauncher))]
    public static class AppLauncherPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("StartProcess")]
        static bool StartProcessPrefix(AppLauncher __instance)
        {
            return !(__instance is AppController);
        }
    }

    public class AppController : AppLauncher
    {
        public AppInstance AppInstance;
        public PlayerServer Player;

        /// <summary>
        /// Set when the launcher receives <c>IdProcess.ScriptEnd</c> and stops reading program messages. Because the
        /// launcher forwards prints as it reads them, the final print has already reached the client.
        /// </summary>
        public readonly ManualResetEventSlim MessagesEnd = new ManualResetEventSlim();

        public AppController(
            ContextManager.ContextProcess process,
            PlayerServer player,
            FilePath filePath,
            string source,
            int windowPid,
            Computer computer,
            GreyInterpreter.HelperConfigInterpreter helperConfigInterpreter
        )
            : base(player, "", "", windowPid, computer, helperConfigInterpreter)
        {
            this.Player = player;

            object launcherProgram = Activator.CreateInstance(RelevantGameMemberTable.MainClass);
            RelevantGameMemberTable.AppLauncherProgram.SetValue(this, launcherProgram);

            AppInstance = new AppInstance();
            AppInstance.Start(
                this,
                process,
                filePath,
                source,
                player.GetPlayerID(),
                computer.GetID(),
                player.GetComputer().GetID(),
                player.playerHelper.GetMapHelper().posNodoPlayer,
                helperConfigInterpreter,
                Hardware.serialCpuSockets,
                interpreter =>
                {
                    RelevantGameMemberTable.MainClassInterpreter.SetValue(
                        launcherProgram,
                        interpreter
                    );
                    process.WriteBinary(interpreter);
                }
            );
            new Thread(() =>
            {
                try
                {
                    RelevantGameMemberTable.AppLauncherListenProcessMessages.Invoke(this, null);
                }
                finally
                {
                    MessagesEnd.Set();
                }
            }).Start();
        }
    }
}
