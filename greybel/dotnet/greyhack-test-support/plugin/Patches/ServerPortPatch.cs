using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using HarmonyLib;

namespace GreyHackTestSupport.Plugin.Patches;

/// <summary>
/// Replaces the fixed single-player server port so multiple game copies can run concurrently. Both server listening
/// (<c>ServerListener.ListenConnections</c>) and local-client connection (<c>PlayerClient.ConnectToServer</c>) use
/// <see cref="Port"/> instead of 21200.
/// </summary>
public static class ServerPortPatch
{
    /// <summary>Hard-coded game server port.</summary>
    private const int GamePort = 21200;

    /// <summary>Server port assigned to this game copy.</summary>
    public static int Port { get; set; } = GamePort;

    private static IEnumerable<CodeInstruction> ReplacePort(
        IEnumerable<CodeInstruction> instructions
    )
    {
        MethodInfo port = AccessTools.PropertyGetter(typeof(ServerPortPatch), nameof(Port));

        foreach (CodeInstruction instruction in instructions)
        {
            if (instruction.LoadsConstant(GamePort))
            {
                yield return new CodeInstruction(OpCodes.Call, port).MoveLabelsFrom(instruction);
            }
            else
            {
                yield return instruction;
            }
        }
    }

    [HarmonyPatch]
    private static class ListenConnectionsPatch
    {
        /// <summary>State-machine <c>MoveNext</c> method containing the async method body.</summary>
        private static MethodBase TargetMethod() =>
            AccessTools.Method(
                AccessTools
                    .Method(typeof(ServerListener), "ListenConnections")
                    .GetCustomAttribute<AsyncStateMachineAttribute>()
                    .StateMachineType,
                "MoveNext"
            );

        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(
            IEnumerable<CodeInstruction> instructions
        ) => ReplacePort(instructions);
    }

    [HarmonyPatch(typeof(PlayerClient), nameof(PlayerClient.ConnectToServer))]
    private static class ConnectToServerPatch
    {
        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(
            IEnumerable<CodeInstruction> instructions
        ) => ReplacePort(instructions);
    }
}
