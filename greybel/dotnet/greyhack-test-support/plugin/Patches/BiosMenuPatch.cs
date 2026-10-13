using System.Collections;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
using UnityEngine;

namespace GreyHackTestSupport.Plugin.Patches;

[HarmonyPatch(typeof(BiosMenu))]
public static class BiosMenuPatch
{
    /// <summary>Player preference set by the menu's "delete computer" option (<c>BiosMenu.UpdateDeletePlayer</c>).</summary>
    private const string DeletePlayerPreference = "user_force_gameover";

    /// <summary>
    /// True when the next start deletes the player computer like the menu's "delete computer" option. The first game
    /// start also deletes it so a broken computer left in the save by an earlier run cannot block startup.
    /// </summary>
    public static bool DeleteComputer { get; set; } = true;

    [HarmonyPostfix]
    [HarmonyPatch("Start")]
    static void Postfix_Start(BiosMenu __instance)
    {
        __instance.StartCoroutine(StartSinglePlayer(__instance));
    }

    static IEnumerator StartSinglePlayer(BiosMenu menu)
    {
        yield return null;
        UnityEngine.Debug.Log("GreyHackTestSupport - starting single player");
        menu.OnSelectSinglePlayer();
        menu.OnStartGame();
    }

    /// <summary>
    /// Supplies <see cref="DeleteComputer"/> instead of the preference when login reads the menu option
    /// (<c>PlayerClientMethods.StartClient</c>). All game copies on a machine share one player-preference file, and each
    /// menu clears this option when opened (<c>BiosMenu.Start</c>, <c>BiosMenu.ResetWipeOptions</c>). Another copy could
    /// otherwise clear it between assignment and login.
    /// </summary>
    private static int TakeDeletePlayer(string key, int defaultValue)
    {
        if (key != DeletePlayerPreference)
        {
            return PlayerPrefs.GetInt(key, defaultValue);
        }

        bool delete = DeleteComputer;

        DeleteComputer = false;

        return delete ? 1 : 0;
    }

    [HarmonyPatch(typeof(PlayerClientMethods), nameof(PlayerClientMethods.StartClient))]
    private static class StartClientPatch
    {
        [HarmonyTranspiler]
        private static IEnumerable<CodeInstruction> Transpiler(
            IEnumerable<CodeInstruction> instructions
        )
        {
            MethodInfo read = AccessTools.Method(
                typeof(PlayerPrefs),
                nameof(PlayerPrefs.GetInt),
                new[] { typeof(string), typeof(int) }
            );
            MethodInfo take = AccessTools.Method(typeof(BiosMenuPatch), nameof(TakeDeletePlayer));

            foreach (CodeInstruction instruction in instructions)
            {
                yield return instruction.Calls(read)
                    ? new CodeInstruction(OpCodes.Call, take).MoveLabelsFrom(instruction)
                    : instruction;
            }
        }
    }
}
