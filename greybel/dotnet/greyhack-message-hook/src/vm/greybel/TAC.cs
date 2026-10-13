using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.CompilerServices;
using Duplex;
using HarmonyLib;
using Miniscript;
using Newtonsoft.Json;

namespace GreyHackMessageHook.Greybel
{
    public enum Op
    {
        Debugger = 1005,
        DebuggerCheck = 1013,
        DebuggerNoop = 1014,
    }

    [HarmonyPatch(typeof(TAC.Line))]
    public static class MiniscriptTacLinePatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("ToString")]
        static bool ToStringPrefix(ref string __result, TAC.Line __instance)
        {
            if ((int)__instance.op < 1000)
            {
                return true;
            }

            Op greybelOp = (Op)__instance.op;

            string text = greybelOp switch
            {
                Op.Debugger => $"{__instance.lhs} := debugger",
                Op.DebuggerCheck => $"{__instance.lhs} := debuggerCheck",
                Op.DebuggerNoop => $"{__instance.lhs} := debuggerNoop",
            };

            if (text != null)
            {
                __result = text;
                return false;
            }

            return false;
        }

        [HarmonyPrefix]
        [HarmonyPatch("Evaluate")]
        static bool EvaluatePrefix(ref Value __result, TAC.Line __instance, TAC.Context context)
        {
            if ((int)__instance.op < 1000)
            {
                return true;
            }

            Op greybelOp = (Op)__instance.op;

            RuntimeMetadata metadata = RuntimeMetadataManager.GetMetadata(context);

            if (metadata == null)
            {
                UnityEngine.Debug.LogWarning(
                    "Skipping op "
                        + greybelOp.ToString()
                        + ". Cannot find metadata at "
                        + __instance.location.context
                        + ":"
                        + __instance.location.lineNum
                );
                __result = ValNull.instance;
                return false;
            }

            RuntimeMetadata.MachineInfo machineInfo = metadata.PeekVM(context.vm);

            if (machineInfo == null)
            {
                UnityEngine.Debug.LogWarning(
                    "Skipping op "
                        + greybelOp.ToString()
                        + ". Cannot find machineInfo at "
                        + __instance.location.context
                        + ":"
                        + __instance.location.lineNum
                );
                __result = ValNull.instance;
                return false;
            }

            ContextManager.ContextProcess contextProcess = ContextManager.GetContext(
                metadata.ContextId
            );

            if (contextProcess == null)
            {
                UnityEngine.Debug.LogWarning(
                    "Skipping op "
                        + greybelOp.ToString()
                        + ". Cannot find contextProcess at "
                        + __instance.location.context
                        + ":"
                        + __instance.location.lineNum
                );
                __result = ValNull.instance;
                return false;
            }

            switch (greybelOp)
            {
                case Op.Debugger:
                    metadata.activeBreakpoint = true;
                    return false;
                case Op.DebuggerNoop:
                    return false;
                case Op.DebuggerCheck:
                {
                    if (__instance.location == null)
                    {
                        return false;
                    }

                    if (metadata.pendingEvaluation != null)
                    {
                        if (metadata.pendingEvaluation.context == context)
                        {
                            context.vm.pause = true;
                            metadata.FinishEvaluation(contextProcess.AppController.Player);
                        }
                        return false;
                    }

                    List<RuntimeMetadata.Breakpoint> breakpoints = contextProcess.Breakpoints;
                    bool atBreakpoint =
                        breakpoints != null
                        && breakpoints.Exists(
                            (breakpoint) =>
                                breakpoint != null
                                && __instance.location.context == breakpoint.FilePath
                                && breakpoint.lineNum == __instance.location.lineNum
                        );
                    bool stepEnds =
                        metadata.stopDepth.HasValue
                        && ContextHelper.GetStack(context).Count <= metadata.stopDepth.Value;

                    bool breakpointOnly = atBreakpoint && !stepEnds && !metadata.activeBreakpoint;

                    if (atBreakpoint || stepEnds)
                    {
                        metadata.activeBreakpoint = true;
                    }

                    if (stepEnds)
                    {
                        metadata.stopDepth = null;
                    }

                    if (metadata.activeBreakpoint)
                    {
                        context.vm.pause = true;

                        Message contextBreakpointRpc = ResponseFactory.CreateContextBreakpoint(
                            metadata.ContextId,
                            __instance.location.context,
                            __instance.location.lineNum,
                            metadata.GetFrames(context),
                            DebugValueTree.ToEntries(context.root.variables),
                            breakpointOnly
                        );

                        PlayerServerEx.SendData(
                            contextBreakpointRpc,
                            contextProcess.AppController.Player
                        );
                    }
                    return false;
                }
            }

            return false;
        }
    }

    public static class ContextHelper
    {
        private static readonly ConditionalWeakTable<TAC.Context, object> forks =
            new ConditionalWeakTable<TAC.Context, object>();

        public static bool IsFork(TAC.Context context)
        {
            return forks.TryGetValue(context, out _);
        }

        public static TAC.Context ForkWithNewCode(TAC.Context context, List<TAC.Line> code)
        {
            TAC.Context newContext = new TAC.Context(code);
            forks.Add(newContext, null);

            if (context.variables == null)
            {
                context.variables = new ValMap();
            }
            newContext.variables = context.variables;
            newContext.outerVars = context.outerVars;
            newContext.self = context.self;
            newContext.args = context.args;
            newContext.parent = context.parent;
            newContext.resultStorage = null;
            newContext.vm = context.vm;
            newContext.partialResult = null;
            newContext.implicitResultCounter = 0;

            SetTemps(newContext, GetTemps(context));

            return newContext;
        }

        public static bool InjectCode(TAC.Context context, string source)
        {
            try
            {
                RuntimeMetadata metadata = RuntimeMetadataManager.GetMetadata(context);
                Parser parser = new Parser(false, metadata.tempNumOffset);
                parser.Parse(source);
                metadata.tempNumOffset = parser.GetCurrentTempNum();
                List<TAC.Line> code = parser.GetCode();

                TAC.Context newContext = ForkWithNewCode(context, code);
                GetStack(context).Push(newContext);
                return true;
            }
            catch (Exception ex)
            {
                Logger.LogError("Injection failed - " + ex.Message);
            }

            return false;
        }

        public static Stack<TAC.Context> GetStack(TAC.Context context)
        {
            return RelevantGameMemberTable.MachineStack.GetValue(context.vm) as Stack<TAC.Context>;
        }

        public static List<Value> GetTemps(TAC.Context context)
        {
            return RelevantGameMemberTable.ContextTemps.GetValue(context) as List<Value>;
        }

        public static void SetTemps(TAC.Context context, List<Value> values)
        {
            RelevantGameMemberTable.ContextTemps.SetValue(context, values);
        }

        public static void DumpTemps(string prefix, TAC.Context context)
        {
            List<Value> temps = GetTemps(context);

            if (temps == null)
            {
                Logger.Log(prefix + " - temps - Failed to get temps");
                return;
            }

            Dictionary<int, string> renderedTemps = new Dictionary<int, string>();

            for (var i = 0; i < temps.Count; i++)
            {
                var item = temps[i];
                if (item == null)
                    continue;
                renderedTemps[i] = item.ToString();
            }

            Logger.Log(prefix + " - temps - " + JsonConvert.SerializeObject(renderedTemps));
        }
    }

    [HarmonyPatch(typeof(TAC.Context))]
    public static class MiniscriptTacContextPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("GetSourceLoc")]
        static bool GetSourceLocPrefix(TAC.Context __instance, ref SourceLoc __result)
        {
            if (__instance.code.Count == 0)
            {
                __result = null;
                return false;
            }
            if (__instance.lineNum < 0)
            {
                __result = __instance.code[0].location;
                return false;
            }
            if (__instance.lineNum >= __instance.code.Count)
            {
                __result = __instance.code[__instance.code.Count - 1].location;
                return false;
            }
            __result = __instance.code[__instance.lineNum].location;
            return false;
        }
    }
}
