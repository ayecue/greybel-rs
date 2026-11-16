using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using BepInEx;
using GreyHackMessageHook.Greybel;
using GreyHackMessageHook.Protocol;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    public static class ContextManager
    {
        public const string RootUser = "root";
        private const string RootHome = "/root";
        private const string UserHome = "/home/";

        public class ContextProcess
        {
            private static readonly System.TimeSpan Compilation = System.TimeSpan.FromSeconds(10);

            /// <summary>Debugger requests executed by the program thread between program steps.</summary>
            private readonly ConcurrentQueue<Action> requests = new ConcurrentQueue<Action>();

            /// <summary>
            /// Schedules <paramref name="request"/> on the program thread between two steps. Debugger requests
            /// access or modify the program VM, which only that thread may do during execution.
            /// </summary>
            public void Post(Action request)
            {
                requests.Enqueue(request);
            }

            /// <summary>Executes requests queued by <see cref="Post"/> on the program thread.</summary>
            public void RunRequests()
            {
                while (requests.TryDequeue(out Action request))
                {
                    try
                    {
                        request();
                    }
                    catch (Exception error)
                    {
                        Logger.LogError("Debugger request failed: " + error);
                    }
                }
            }

            public int Pid = -1;
            public FilePath FilePath;
            public string BasePath;

            /// <summary>Binary program code built from the program files.</summary>
            public string Source;

            /// <summary>Original source locations for lines in <see cref="Source"/>.</summary>
            public SourceMap SourceMap;

            /// <summary>
            /// True when the game compiles <see cref="Source"/> as strict GreyScript, where <c>debugger</c> is an
            /// identifier rather than a breakpoint.
            /// </summary>
            public bool StrictMode;
            public List<string> Params;
            public string ProgramName;
            public Computer Computer;
            public string ContextId;
            public bool DebugMode;
            public List<RuntimeMetadata.Breakpoint> Breakpoints;
            public GreyInterpreter.HelperConfigInterpreter HelperConfigInterpreter;
            public AppController AppController;

            /// <summary>Terminal for the windowless program.</summary>
            public ContextTerminal Terminal;
            public HelperScript HelperScript;
            public PlayerServer Player;
            public string ActiveUser;

            /// <summary>
            /// True when crashes include the call stack for editor runs. The game terminal prints only the error
            /// (<c>Interpreter.ReportError</c>).
            /// </summary>
            public bool PrintsCallStack;

            /// <summary>
            /// True when the program runs in the safe-mode terminal (<c>GreyInterpreter.isSafeTerminal</c>), whose
            /// <c>launch</c> doesn't need the file's execute permission.
            /// </summary>
            public bool SafeTerminal;

            public ContextProcess(
                string contextId,
                bool debugMode,
                PlayerServer player,
                string basePath,
                string filePath,
                string programName,
                string source,
                SourceMap sourceMap,
                bool strictMode,
                List<string> parameters,
                Computer computer,
                List<RuntimeMetadata.Breakpoint> breakpoints,
                string activeUser = RootUser,
                bool printsCallStack = true,
                bool safeTerminal = false
            )
            {
                this.PrintsCallStack = printsCallStack;
                this.SafeTerminal = safeTerminal;
                this.SourceMap = sourceMap;
                this.StrictMode = strictMode;
                this.Params = parameters;
                this.ContextId = contextId;
                this.DebugMode = debugMode;
                this.Player = player;
                this.FilePath = new FilePath(filePath);
                this.BasePath = basePath;
                this.ProgramName = programName;
                this.Breakpoints = breakpoints;
                this.Source = source;
                this.Computer = computer;
                this.ActiveUser = activeUser;
            }

            public void Start()
            {
                if (Pid != -1)
                {
                    return;
                }

                string home = Home;
                Pid = Computer.AddProcess(
                    ProgramName,
                    ActiveUser,
                    0,
                    isScript: true,
                    isTerminal: false,
                    Computer.GetID(),
                    -1,
                    true
                );
                HelperConfigInterpreter = new GreyInterpreter.HelperConfigInterpreter(
                    Pid,
                    ActiveUser,
                    home,
                    $"{home}/{ProgramName}",
                    $"{home}/{ProgramName}",
                    Computer.GetPublicIP(),
                    0,
                    null,
                    false,
                    SafeTerminal,
                    Pid
                );
                Terminal = new ContextTerminal(Pid, home, ActiveUser);
                AppController = new AppController(
                    this,
                    Player,
                    FilePath,
                    Source,
                    Pid,
                    Computer,
                    HelperConfigInterpreter
                );
                HelperScript = new HelperScript(Pid);
                HelperScript.SetAppLauncher(AppController);

                Player.greyScriptHelper.helperScript.TryAdd(Pid, HelperScript);
            }

            /// <summary>Program home directory.</summary>
            private string Home => ActiveUser == RootUser ? RootHome : $"{UserHome}{ActiveUser}";

            /// <summary>Path of the binary executed by the program.</summary>
            public string BinaryPath => $"{Home}/{ProgramName}";

            /// <summary>True when the plugin created the program binary and must delete it after the run.</summary>
            private bool binaryWritten;

            /// <summary>
            /// Creates the program binary at <see cref="BinaryPath"/> using the game <c>build</c> behavior so the
            /// program can launch itself. It replaces an existing binary but retains other file types unchanged.
            /// </summary>
            public void WriteBinary(GreyInterpreter interpreter)
            {
                FileSystem.Fichero current = Computer
                    .GetFileSystem()
                    .GetFichero(BinaryPath, includeSymLinks: false);

                if (current != null)
                {
                    if (
                        !(current is FileSystem.Archivo file)
                        || !file.IsBinario()
                        || !file.IsTypeFile(FileSystem.Fichero.TypeFile.Script)
                    )
                    {
                        Logger.LogError(
                            "WriteBinary - "
                                + BinaryPath
                                + " isn't a binary: the program can't launch itself"
                        );
                        return;
                    }

                    RemoveBinary();
                }

                string error = PlayerUtils.CrearArchivoSyncClients(
                    Home,
                    ProgramName,
                    Source,
                    ProgramName,
                    "",
                    ActiveUser,
                    isBinario: true,
                    isLibrary: false,
                    isScript: true,
                    isLog: false,
                    Computer,
                    interpreter
                );

                if (!string.IsNullOrEmpty(error))
                {
                    Logger.LogError("WriteBinary - " + error);
                    return;
                }

                binaryWritten = true;
                ProgramBinaryTable.Register(Source, new ProgramBinary(SourceMap, StrictMode));
            }

            /// <summary>
            /// Deletes <see cref="BinaryPath"/> without producing the log written by script deletion. The computer is
            /// read again: <see cref="Computer"/> is the copy the program began with, and saving its file system
            /// would undo every change the program made to the files, folders and users of the computer since.
            /// </summary>
            private void RemoveBinary()
            {
                FileSystem.Fichero initial = Computer
                    .GetFileSystem()
                    .GetFichero(BinaryPath, includeSymLinks: false);
                FileSystem.Carpeta initialParent = initial?.GetParent();
                if (initialParent != null)
                {
                    Computer.GetFileSystem().RemoveFichero(initial, initialParent.GetRuta());
                }

                Computer current = ServerMap.Singleton.GetRemoteComputer(Computer.GetID());
                if (current == null)
                {
                    return;
                }

                FileSystem.Fichero binary = current
                    .GetFileSystem()
                    .GetFichero(BinaryPath, includeSymLinks: false);
                FileSystem.Carpeta parent = binary?.GetParent();

                if (
                    parent != null
                    && current.GetFileSystem().RemoveFichero(binary, parent.GetRuta())
                )
                {
                    Database.Singleton.SyncFileSystemDB(current, onlyFileSystem: true);
                }
            }

            public void Finish(bool failed)
            {
                if (Pid == -1)
                {
                    return;
                }

                PlayerServerEx.SendData(new FinishedContextRpc(ContextId, failed), Player);
                DisposeContext(ContextId);
            }

            public void Dispose()
            {
                if (Pid == -1)
                {
                    return;
                }

                ContextTerminal terminal = Terminal;

                ThreadingHelper.Instance.StartSyncInvoke(() => terminal.Cancel());

                Greybel.Interpreter interpreter = AppController.AppInstance.WaitForCompilation(
                    Compilation
                );
                if (interpreter != null)
                {
                    // Read the computer again: Computer.SyncProcesos saves the complete computer while its configuration
                    // isn't saved yet, and the copy the program began with would undo every change it made since.
                    Computer current =
                        ServerMap.Singleton.GetRemoteComputer(Computer.GetID()) ?? Computer;
                    current.CloseProgram(Pid, interpreter, true);
                }
                AppController.KillScript(isTerminal: true);
                HelperScript.FinishMining();
                Player.greyScriptHelper.helperScript.TryRemove(Pid, out var _);
                if (interpreter != null)
                {
                    RuntimeMetadataManager.Dispose(interpreter);
                }
                if (binaryWritten)
                {
                    ProgramBinaryTable.Unregister(Source);
                    RemoveBinary();
                    binaryWritten = false;
                }
                Pid = -1;
            }
        }

        private static readonly ConcurrentDictionary<string, ContextProcess> contextRegistry =
            new ConcurrentDictionary<string, ContextProcess>();

        public static string CreateContext(
            string id,
            bool debugMode,
            PlayerServer player,
            string basePath,
            string filePath,
            string programName,
            string source,
            SourceMap sourceMap,
            bool strictMode,
            List<string> parameters,
            Computer computer,
            List<RuntimeMetadata.Breakpoint> breakpoints,
            string activeUser = RootUser,
            bool printsCallStack = true,
            bool safeTerminal = false
        )
        {
            ContextProcess process = new ContextProcess(
                id,
                debugMode,
                player,
                basePath,
                filePath,
                programName,
                source,
                sourceMap,
                strictMode,
                parameters,
                computer,
                breakpoints,
                activeUser,
                printsCallStack,
                safeTerminal
            );

            // AppInstance.Start may precede the process.Start return, so register its terminal first.
            if (!contextRegistry.TryAdd(id, process))
            {
                throw new InvalidOperationException($"Context already exists: {id}");
            }

            process.Start();

            return id;
        }

        public static ContextProcess GetContext(string id)
        {
            ContextProcess process = null;
            contextRegistry.TryGetValue(id, out process);
            return process;
        }

        public static ContextProcess GetContextByPid(int pid)
        {
            ContextProcess process = contextRegistry.Values.FirstOrDefault(process =>
                process.Pid == pid
            );

            return process;
        }

        public static bool DisposeContext(string id)
        {
            ContextProcess process = null;
            contextRegistry.TryGetValue(id, out process);

            if (process == null)
            {
                return false;
            }

            process.Dispose();

            return contextRegistry.TryRemove(id, out _);
        }
    }
}
