using System;
using System.Collections.Generic;
using System.IO;
using GreyHackMessageHook.Protocol;
using HarmonyLib;
using Miniscript;
using NetworkMessages;
using Newtonsoft.Json;
using static HelperScript;

namespace GreyHackMessageHook.Greybel
{
    public class RuntimeMetadata
    {
        public class StackItem
        {
            public readonly string name;

            [JsonProperty("filepath")]
            public readonly string FilePath;
            public readonly int lineNum;
            public readonly bool isExternal;

            public StackItem(string name, string filePath, int lineNum, bool isExternal)
            {
                this.name = name;
                this.FilePath = filePath;
                this.lineNum = lineNum;
                this.isExternal = isExternal;
            }
        }

        public class MachineInfo
        {
            public readonly RuntimeMetadata metadata;
            public readonly TAC.Machine vm;

            public MachineInfo(RuntimeMetadata metadata, TAC.Machine vm)
            {
                this.metadata = metadata;
                this.vm = vm;
            }

            private TAC.Line GetCurrentCode(TAC.Context context)
            {
                if (context.code.Count == 0)
                {
                    return null;
                }

                int idx = context.done ? context.lineNum - 1 : context.lineNum;
                TAC.Line line = context.code[idx];

                if ((int)line.op == (int)Op.DebuggerCheck)
                {
                    idx = context.lineNum + 1;
                }

                if (idx < 0)
                {
                    return context.code[0];
                }
                else if (idx >= context.code.Count)
                {
                    return context.code[context.code.Count - 1];
                }

                return context.code[idx];
            }

            public List<StackItem> GetStackTrace()
            {
                List<StackItem> result = new List<StackItem>();
                Stack<TAC.Context> vmStack =
                    RelevantGameMemberTable.MachineStack.GetValue(vm) as Stack<TAC.Context>;

                foreach (TAC.Context context in vmStack)
                {
                    TAC.Line line = GetCurrentCode(context);
                    string opName;

                    if ((int)line.op < 1000)
                    {
                        opName = Enum.GetName(typeof(TAC.Line.Op), (int)line.op);
                    }
                    else
                    {
                        opName = Enum.GetName(typeof(Op), (int)line.op);
                    }

                    var loc = context.GetSourceLoc();
                    if (loc == null)
                        continue;
                    result.Add(
                        new StackItem(
                            opName,
                            loc.context,
                            loc.lineNum,
                            metadata.IsProgramFile(loc.context)
                        )
                    );
                }

                return result;
            }
        }

        public class Breakpoint
        {
            [JsonProperty("filepath")]
            public string FilePath;
            public int lineNum;
        }

        public int tempNumOffset = 0;
        public readonly Miniscript.Interpreter Interpreter;
        public readonly string ContextId;
        public readonly Dictionary<TAC.Machine, MachineInfo> machines =
            new Dictionary<TAC.Machine, MachineInfo>();

        public bool activeBreakpoint = false;

        public int? stopDepth = null;

        public Evaluation pendingEvaluation = null;

        public class Evaluation
        {
            public const string Variable = "__greybel_evaluation";

            public TAC.Context context;
            public bool expression;
        }

        public class DebugFrame
        {
            public string name;

            [JsonProperty("filepath")]
            public string FilePath;
            public int lineNum;
            public List<DebugValueTree.Entry> locals;
        }

        public List<DebugFrame> GetFrames(TAC.Context context)
        {
            var frames = new List<DebugFrame>();
            TAC.Context[] contexts = ContextHelper.GetStack(context).ToArray();
            for (int index = 0; index < contexts.Length; index++)
            {
                TAC.Context frame = contexts[index];

                int line = frame.lineNum - 1;
                SourceLoc location =
                    line >= 0 && line < frame.code.Count
                        ? frame.code[line].location
                        : frame.GetSourceLoc();
                if (location == null)
                    continue;
                bool isFile = index == contexts.Length - 1 || ContextHelper.IsFork(frame);
                SourceLoc start = frame.code.Count > 0 ? frame.code[0].location : null;
                frames.Add(
                    new DebugFrame
                    {
                        name =
                            isFile || start == null
                                ? GetFileName(location.context)
                                : "function " + GetFileName(start.context) + ":" + start.lineNum,
                        FilePath = location.context,
                        lineNum = location.lineNum,
                        locals = DebugValueTree.ToEntries(frame.variables),
                    }
                );
            }
            return frames;
        }

        private static string GetFileName(string filePath)
        {
            if (filePath == null)
                return "";
            int slash = filePath.LastIndexOf('/');
            return slash == -1 ? filePath : filePath.Substring(slash + 1);
        }

        public string StartEvaluation(TAC.Context context, string code)
        {
            List<TAC.Line> lines;
            bool expression = true;
            try
            {
                lines = Compile(Evaluation.Variable + " = (" + code + ")");
            }
            catch (MiniscriptException)
            {
                expression = false;
                try
                {
                    lines = Compile(code);
                }
                catch (MiniscriptException ex)
                {
                    return ex.Message;
                }
            }
            TAC.Context evaluation = ContextHelper.ForkWithNewCode(context, lines);
            ContextHelper.GetStack(context).Push(evaluation);
            context.lineNum--;
            pendingEvaluation = new Evaluation { context = context, expression = expression };
            return null;
        }

        private List<TAC.Line> Compile(string code)
        {
            Parser parser = new Parser(false, tempNumOffset);
            parser.Parse(code);
            tempNumOffset = parser.GetCurrentTempNum();
            return parser.GetCode();
        }

        public void FinishEvaluation(PlayerServer player)
        {
            Evaluation evaluation = pendingEvaluation;
            pendingEvaluation = null;
            byte[] value = new byte[0];
            bool hasValue = false;
            ValMap variables = evaluation.context.variables;
            var key = new ValString(Evaluation.Variable);
            if (
                evaluation.expression
                && variables != null
                && variables.map.TryGetValue(key, out Value result)
            )
            {
                variables.map.Remove(key);
                value = GreyHackMessageHook.Game.StringCompressor.Zip(
                    Newtonsoft.Json.JsonConvert.SerializeObject(DebugValueTree.ToNode(result))
                );
                hasValue = true;
            }
            PlayerServerEx.SendData(
                new ContextEvaluatedRpc(ContextId, false, "", hasValue, value),
                player
            );
        }

        public bool FailEvaluation(string error, PlayerServer player)
        {
            Evaluation evaluation = pendingEvaluation;
            if (evaluation == null)
            {
                return false;
            }
            pendingEvaluation = null;
            Stack<TAC.Context> machines = ContextHelper.GetStack(evaluation.context);
            while (machines.Count > 0 && machines.Peek() != evaluation.context)
            {
                machines.Pop();
            }
            evaluation.context.lineNum++;
            evaluation.context.variables?.map.Remove(new ValString(Evaluation.Variable));
            evaluation.context.vm.pause = true;
            PlayerServerEx.SendData(
                new ContextEvaluatedRpc(ContextId, true, error, false, new byte[0]),
                player
            );
            return true;
        }

        public RuntimeMetadata(string contextId, Miniscript.Interpreter interpreter)
        {
            this.ContextId = contextId;
            this.Interpreter = interpreter;
            AddVM(interpreter.vm);
        }

        public RuntimeMetadata(
            string contextId,
            Miniscript.Interpreter interpreter,
            string filePath
        )
        {
            this.ContextId = contextId;
            this.Interpreter = interpreter;
            this.FilePath = filePath;
            AddVM(interpreter.vm);
        }

        /// <summary>Program entry-point file.</summary>
        public readonly string FilePath;

        /// <summary>True when <paramref name="path"/> is the program entry point or a file named by its source map.</summary>
        public bool IsProgramFile(string path)
        {
            if (path == FilePath)
            {
                return true;
            }

            SourceMap sourceMap = ContextManager.GetContext(ContextId)?.SourceMap;

            return sourceMap != null && sourceMap.Sources.Contains(path);
        }

        public void AddVM(TAC.Machine vm)
        {
            machines[vm] = new MachineInfo(this, vm);
        }

        public MachineInfo PeekVM(TAC.Machine vm)
        {
            if (!machines.ContainsKey(vm))
            {
                return null;
            }

            return machines[vm];
        }

        public List<StackItem> GetStackTrace()
        {
            List<StackItem> result = new List<StackItem>();

            if (Interpreter.vm == null)
            {
                return result;
            }

            List<TAC.Machine> pendingVms = (
                RelevantGameMemberTable.HelperScriptVmList.GetValue(Interpreter.hostData)
                as List<HelperVmState>
            ).ConvertAll<TAC.Machine>(state => state.vm);
            List<TAC.Machine> vmList = new List<TAC.Machine>() { Interpreter.vm };

            vmList.AddRange(pendingVms);

            foreach (TAC.Machine vm in vmList)
            {
                var metadataRoot = machines[vm];
                result.AddRange(metadataRoot.GetStackTrace());
            }

            return result;
        }
    }

    public static class RuntimeMetadataManager
    {
        public static Dictionary<Miniscript.Interpreter, RuntimeMetadata> Registry =
            new Dictionary<Miniscript.Interpreter, RuntimeMetadata>();

        public static void Register(Miniscript.Interpreter interpreter, RuntimeMetadata metadata)
        {
            if (Registry.ContainsKey(interpreter))
            {
                return;
            }

            Registry[interpreter] = metadata;
        }

        public static RuntimeMetadata GetMetadata(Miniscript.Interpreter interpreter)
        {
            RuntimeMetadata result = null;
            Registry.TryGetValue(interpreter, out result);
            return result;
        }

        public static RuntimeMetadata GetMetadata(TAC.Machine vm)
        {
            RuntimeMetadata result = null;
            Registry.TryGetValue((Miniscript.Interpreter)vm.interpreter.Target, out result);
            return result;
        }

        public static RuntimeMetadata GetMetadata(TAC.Context context)
        {
            RuntimeMetadata result = null;
            Registry.TryGetValue((Miniscript.Interpreter)context.vm.interpreter.Target, out result);
            return result;
        }

        public static RuntimeMetadata.MachineInfo GetMachineInfo(Miniscript.Interpreter interpreter)
        {
            var metadataRoot = GetMetadata(interpreter);
            return metadataRoot?.PeekVM(interpreter.vm);
        }

        public static RuntimeMetadata.MachineInfo GetMachineInfo(TAC.Machine vm)
        {
            var interpreter = (Miniscript.Interpreter)vm.interpreter.Target;
            var metadataRoot = GetMetadata(interpreter);
            return metadataRoot?.PeekVM(vm);
        }

        public static bool Dispose(Miniscript.Interpreter interpreter)
        {
            if (!Registry.ContainsKey(interpreter))
            {
                return false;
            }
            Registry.Remove(interpreter);
            return true;
        }
    }

    [HarmonyPatch(typeof(Miniscript.Interpreter))]
    public static class MiniscriptInterpreterPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("Compile")]
        static bool CompilePrefix(GreyInterpreter __instance)
        {
            Logger.Log("Compile - Start");
            if (__instance.vm != null)
            {
                return false;
            }
            string source =
                RelevantGameMemberTable.InterpreterSource.GetValue(__instance) as string;
            Interpreter greybelInstance = __instance as Interpreter;
            bool program = greybelInstance != null && !greybelInstance.ProgramCompiled;
            ContextManager.ContextProcess process = program
                ? ContextManager.GetContext(greybelInstance.ContextId)
                : null;
            ProgramBinary binary = program
                ? new ProgramBinary(process?.SourceMap, process?.StrictMode ?? false)
                : ProgramBinaryTable.Find(source);

            // Compile non-Greybel code with game behavior. Greybel code includes the current program, programs it
            // launches, and copies of its binary.
            if (greybelInstance == null && binary == null)
            {
                return true;
            }

            Parser currentParser =
                RelevantGameMemberTable.InterpreterParser.GetValue(__instance) as Parser;
            if (currentParser == null)
            {
                currentParser =
                    greybelInstance != null ? new Parser(greybelInstance.DebugMode) : new Parser();
                RelevantGameMemberTable.InterpreterParser.SetValue(__instance, currentParser);
            }
            currentParser.sourceMap = binary?.SourceMap;
            currentParser.isExternal = program;
            currentParser.debuggerStatements = binary != null && !binary.StrictMode;
            if (currentParser.isExternal)
            {
                currentParser.FilePath = greybelInstance.ProgramFilePath;
                greybelInstance.ProgramCompiled = true;
            }
            try
            {
                // Attribute current-program lines to their source file. Attribute launched-program lines to the
                // game-generated lines that replaced imports, matching game behavior.
                if (currentParser.isExternal)
                {
                    currentParser.Parse(source, replMode: false);
                }
                else
                {
                    currentParser.Parse(source, replMode: false, __instance.helperImport);
                }
                TAC.Machine vm = currentParser.CreateVM(__instance.standardOutput);
                vm.interpreter = new WeakReference(__instance);
                __instance.vm = vm;

                if (__instance is Interpreter greybelInterpreterInstance)
                {
                    RuntimeMetadata metadata = RuntimeMetadataManager.GetMetadata(__instance);

                    if (metadata != null)
                    {
                        metadata.tempNumOffset = currentParser.GetCurrentTempNum();
                        metadata.AddVM(__instance.vm);
                        Logger.Log("Compile - Register to existing greybel related metadata");

                        if (!currentParser.isExternal)
                        {
                            ContextManager.ContextProcess contextProcess =
                                ContextManager.GetContext(metadata.ContextId);
                            PlayerServerEx.SendData(
                                ResponseFactory.CreateContextOpenInternalFile(
                                    metadata.ContextId,
                                    currentParser.FilePath,
                                    source
                                ),
                                contextProcess.Player
                            );
                        }
                    }
                    else
                    {
                        metadata = new RuntimeMetadata(
                            greybelInterpreterInstance.ContextId,
                            __instance,
                            currentParser.FilePath
                        );
                        metadata.tempNumOffset = currentParser.GetCurrentTempNum();

                        Logger.Log("Compile - Register new greybel related metadata");
                        RuntimeMetadataManager.Register(__instance, metadata);
                    }
                }
            }
            catch (MiniscriptException error)
            {
                if (error is CompilerException compilerException)
                {
                    Logger.LogError(
                        "Compile - Exception "
                            + error.Message
                            + " at line "
                            + compilerException.location.lineNum
                    );
                }
                else
                {
                    Logger.LogError("Compile - Exception: " + error.Message);
                }
                RelevantGameMemberTable.InterpreterReportError.Invoke(
                    __instance,
                    new object[] { error }
                );
                if (__instance.vm == null)
                {
                    RelevantGameMemberTable.InterpreterParser.SetValue(__instance, null);
                }
            }

            return false;
        }
    }

    [HarmonyPatch(typeof(GreyInterpreter))]
    public static class GreyInterpreterPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("SendMessage")]
        static bool SendMessagePrefix(GreyInterpreter __instance, MessageProcess messageProcess)
        {
            StreamWriter writer =
                RelevantGameMemberTable.InterpreterWriter.GetValue(__instance) as StreamWriter;

            if (writer.BaseStream == null)
            {
                Logger.Log("Writer is inactive. Therefore blocking any communication");
                return false;
            }
            return true;
        }
    }

    public class Interpreter : GreyInterpreter
    {
        public string ContextId;
        public bool DebugMode;

        /// <summary>In-game path of the program entry file.</summary>
        public readonly string ProgramFilePath;

        /// <summary>True while compiling the original program; subsequent compilation belongs to launched code.</summary>
        public bool ProgramCompiled;

        /// <summary>Program <c>params</c>, copied to global <c>params</c> after compilation.</summary>
        public List<string> Params;
        public List<SourceLoc> lastStackTrace;

        /// <summary>True when <see cref="ReportError"/> appends the call stack to the error.</summary>
        public bool PrintsCallStack;

        public Interpreter(
            string contextId,
            bool debugMode,
            FilePath filePath,
            string source,
            string playerId,
            string computerId,
            string playerComputerId,
            CustomVector.Vector2 posNodoPlayer,
            HelperConfigInterpreter helperConfigInterpreter,
            string serialCpuSockets,
            string defaultBashId
        )
            : base(
                source,
                playerId,
                computerId,
                playerComputerId,
                posNodoPlayer,
                helperConfigInterpreter,
                serialCpuSockets,
                defaultBashId
            )
        {
            this.DebugMode = debugMode;
            this.ProgramFilePath = filePath.ToFullPath();
            this.ContextId = contextId;
        }

        public override void RunUntilDone(double timeLimit, bool returnEarly)
        {
            try
            {
                if (vm == null)
                {
                    Compile();
                    if (vm == null)
                    {
                        return;
                    }
                    if (Params != null)
                    {
                        vm.globalContext.SetVar(
                            "params",
                            new ValList(Params.ConvertAll(param => (Value)new ValString(param)))
                        );
                        Params = null;
                    }
                }
                double runTime = vm.runTime;
                vm.yielding = false;
                while (!vm.done && !vm.yielding && !vm.pause && !(vm.runTime - runTime > timeLimit))
                {
                    vm.Step();
                    lastStackTrace = vm.GetStack();
                    if (returnEarly && vm.GetTopContext().partialResult != null)
                    {
                        break;
                    }
                }
            }
            catch (Exception ex)
            {
                if (ex is MiniscriptException evaluationError && FailEvaluation(evaluationError))
                {
                    return;
                }
                hostData.scriptCrash = true;
                if (ex is MiniscriptException error)
                {
                    ReportError(error);
                    Stop();
                    return;
                }
                if (ex is OutOfMemoryException)
                {
                    hostData.CancelScript();
                    return;
                }
                _ = currentPath;
                UnityEngine.Debug.LogException(ex);
                if (vm != null)
                {
                    vm.Stop();
                }
            }
        }

        private bool FailEvaluation(MiniscriptException error)
        {
            RuntimeMetadata metadata = RuntimeMetadataManager.GetMetadata(this);
            ContextManager.ContextProcess process = ContextManager.GetContext(ContextId);
            return metadata != null
                && process != null
                && metadata.FailEvaluation(error.Description(), process.Player);
        }

        protected override void ReportError(MiniscriptException error)
        {
            errorOutput(error.Description());

            if (PrintsCallStack && lastStackTrace != null)
            {
                foreach (var loc in lastStackTrace)
                {
                    if (loc == null)
                        continue;
                    errorOutput("  " + loc.context + ":" + loc.lineNum);
                }
            }
        }
    }
}
