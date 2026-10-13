using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using HarmonyLib;
using Miniscript;
using NetworkMessages;
using Util;

namespace GreyHackMessageHook
{
    public static class RelevantGameMemberTable
    {
        public static readonly FieldInfo GameVersionField = AccessTools.Field(
            typeof(GameConfig),
            "gameVersion"
        );

        public static readonly FieldInfo PlayerServerStream = AccessTools.Field(
            typeof(PlayerServer),
            "stream"
        );
        public static readonly FieldInfo PlayerServerLockWrite = AccessTools.Field(
            typeof(PlayerServer),
            "lockWrite"
        );
        public static readonly FieldInfo PlayerServerSocketClose = AccessTools.Field(
            typeof(PlayerServer),
            "socketClosing"
        );
        public static readonly MethodInfo PlayerServerProcessReceivedData = AccessTools.Method(
            typeof(PlayerServer),
            "ProcessReceivedData",
            new[] { typeof(MessageServer) }
        );

        public static readonly FieldInfo PlayerClientClient = AccessTools.Field(
            typeof(PlayerClient),
            "client"
        );
        public static readonly FieldInfo PlayerClientStream = AccessTools.Field(
            typeof(PlayerClient),
            "stream"
        );
        public static readonly MethodInfo PlayerClientProcessReceivedData = AccessTools.Method(
            typeof(PlayerClient),
            "ProcessReceivedData",
            new[] { typeof(MessageClient) }
        );

        public static readonly FieldInfo MachineStack = AccessTools.Field(
            typeof(TAC.Machine),
            "stack"
        );
        public static readonly FieldInfo ContextTemps = AccessTools.Field(
            typeof(TAC.Context),
            "temps"
        );

        public static readonly FieldInfo InterpreterParser = AccessTools.Field(
            typeof(GreyInterpreter),
            "parser"
        );
        public static readonly FieldInfo InterpreterSource = AccessTools.Field(
            typeof(GreyInterpreter),
            "source"
        );
        public static readonly FieldInfo InterpreterWriter = AccessTools.Field(
            typeof(GreyInterpreter),
            "writer"
        );
        public static readonly MethodInfo InterpreterReportError = AccessTools.Method(
            typeof(GreyInterpreter),
            "ReportError",
            new[] { typeof(MiniscriptException) }
        );

        public static readonly FieldInfo HelperScriptVmList = AccessTools.Field(
            typeof(HelperScript),
            "vmList"
        );

        public static readonly FieldInfo ProgramWindowVentana = AccessTools.Field(
            typeof(ProgramVisual.ProgramWindow),
            "ventana"
        );

        public static readonly Type MainClass = AccessTools.TypeByName("MainClass");
        public static readonly FieldInfo MainClassInterpreter =
            MainClass == null ? null : AccessTools.Field(MainClass, "interpreter");
        public static readonly FieldInfo AppLauncherProgram = AccessTools.Field(
            typeof(AppLauncher),
            "program"
        );
        public static readonly MethodInfo AppLauncherListenProcessMessages = AccessTools.Method(
            typeof(AppLauncher),
            "ListenProcessMessages"
        );

        public static IReadOnlyList<string> GetMissingMembers()
        {
            return typeof(RelevantGameMemberTable)
                .GetFields(BindingFlags.Public | BindingFlags.Static)
                .Where(field => field.GetValue(null) == null)
                .Select(field => field.Name)
                .ToList();
        }
    }
}
