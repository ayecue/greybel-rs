using System.Diagnostics;
using System.Globalization;
using System.Threading;
using CPULimiter;
using Miniscript;
using NetworkMessages;
using static GreyInterpreter;

namespace GreyHackMessageHook.Greybel
{
    public class AppInstance
    {
        private Thread scriptThread;

        private readonly ManualResetEventSlim compilation = new ManualResetEventSlim(false);

        public Interpreter Interpreter;

        /// <summary>True when the program crashed; assigned before termination with <c>IdProcess.ScriptEnd</c>.</summary>
        public bool Failed { get; private set; }

        /// <summary>Maximum time program termination waits for the launcher to forward remaining messages.</summary>
        private static readonly System.TimeSpan ListenTime = System.TimeSpan.FromSeconds(5);

        public Interpreter WaitForCompilation(System.TimeSpan timeout)
        {
            compilation.Wait(timeout);
            return Interpreter;
        }

        public void Start(
            AppLauncher launcher,
            ContextManager.ContextProcess process,
            FilePath filePath,
            string source,
            string playerId,
            string computerId,
            string playerComputerId,
            CustomVector.Vector2 posNodoPlayer,
            HelperConfigInterpreter helperConfigInterpreter,
            string serialCpuSockets,
            System.Action<Interpreter> onInterpreter
        )
        {
            Thread.CurrentThread.CurrentCulture = CultureInfo.InvariantCulture;
            HostInfo.name = "Greyscript";
            string sourceBash = XmlGlobal.CacheText.configGeneration["bash"];
            string defaultBashId = Computer.Md5Sum(sourceBash);
            scriptThread = new Thread(
                (ThreadStart)
                    delegate
                    {
                        try
                        {
                            Run(
                                launcher,
                                process,
                                filePath,
                                source,
                                playerId,
                                computerId,
                                playerComputerId,
                                posNodoPlayer,
                                helperConfigInterpreter,
                                serialCpuSockets,
                                defaultBashId,
                                onInterpreter
                            );
                        }
                        finally
                        {
                            compilation.Set();
                        }
                    }
            );
            scriptThread.Start();
        }

        private void Run(
            AppLauncher launcher,
            ContextManager.ContextProcess process,
            FilePath filePath,
            string source,
            string playerId,
            string computerId,
            string playerComputerId,
            CustomVector.Vector2 posNodoPlayer,
            HelperConfigInterpreter helperConfigInterpreter,
            string serialCpuSockets,
            string defaultBashId,
            System.Action<Interpreter> onInterpreter
        )
        {
            Interpreter = new Interpreter(
                process.ContextId,
                process.DebugMode,
                filePath,
                source,
                playerId,
                computerId,
                playerComputerId,
                posNodoPlayer,
                helperConfigInterpreter,
                serialCpuSockets,
                defaultBashId
            );
            Interpreter.Params = process.Params;
            Interpreter.PrintsCallStack = process.PrintsCallStack;
            onInterpreter(Interpreter);
            Interpreter.hostData.SetAppLauncher(launcher);
            // MainClass.Main assigns the player's pending shell to a program running off the player computer.
            if (computerId != playerComputerId)
            {
                Computer hostComputer = ServerMap.Singleton.GetRemoteComputer(computerId);
                string publicIp = hostComputer.GetPublicIP();
                Shell shell = new Shell(
                    hostComputer.GetID(),
                    Interpreter.activeUser,
                    publicIp,
                    publicIp,
                    hostComputer.GetLocalIP(),
                    hostComputer.GetNombreMaquina(),
                    Interpreter.startNumBounces,
                    Interpreter.prevConnIp
                );
                Interpreter.hostData.AddPendingShell(shell);
            }
            Stopwatch stopwatch = new Stopwatch();
            bool wasPaused = false;
            CpuUsage cpuUsage = new CpuUsage();
            do
            {
                Interpreter.RunUntilDone(0.01, returnEarly: true);
                compilation.Set();
                process?.RunRequests();
                Thread.Yield();
                Interpreter.SendPendingPrint();
                Interpreter.SendPendingSniffers();
                Interpreter.ShouldSendFolders(stopwatch);
                if (Interpreter.Paused() && !wasPaused && !Interpreter.hostData.canceled)
                {
                    wasPaused = true;
                }
                if (wasPaused && Interpreter.hostData.canceled)
                {
                    wasPaused = false;
                    Interpreter.Resume();
                }
                if (!Interpreter.Running() && Interpreter.hostData.IsPendingVM())
                {
                    // Save the launched program's computer before replacing it with the launching program's state.
                    string childComputerId = Interpreter.computerID;
                    HelperScript.HelperVmState lastVm = Interpreter.hostData.GetLastVM();
                    int bouncesBeforeExit = Interpreter.GetStartNumBounces();
                    Interpreter.SetVmState(lastVm);
                    // MainClass: ending a terminal's bash with `exit`, or a connection interrupted or a reboot,
                    // closes the programs of the remote it was on (`flag4`).
                    bool endedTerminal =
                        Interpreter.exitingBash
                        || Interpreter.interruptConnection
                        || Interpreter.exitBashReboot;
                    Interpreter.exitingBash = false;
                    Interpreter.exitBashReboot = false;
                    Interpreter.interruptConnection = false;
                    // Match the game terminal (MainClass): a crashed launched program resets the custom object, and
                    // the launching program continues with non-crash state.
                    if (Interpreter.hostData.scriptCrash)
                    {
                        Interpreter.hostData.ResetCustomObject();
                        Interpreter.hostData.scriptCrash = false;
                    }
                    // Match MainClass by terminating the launched program's process with the program.
                    if (Interpreter.hostData.subPID > 0)
                    {
                        using (new FilesystemLock(childComputerId, Interpreter.GetScriptID()))
                        {
                            Computer remoteComputer = Interpreter.hostData.GetRemoteComputer(
                                childComputerId,
                                partialObj: false,
                                checkConnection: false
                            );
                            if (remoteComputer != null)
                            {
                                remoteComputer.CloseProgram(
                                    Interpreter.hostData.subPID,
                                    Interpreter,
                                    scriptFinished: true
                                );
                                if (endedTerminal)
                                {
                                    Interpreter.hostData.CloseRemotePrograms(
                                        remoteComputer,
                                        bouncesBeforeExit
                                    );
                                }
                            }
                            Interpreter.hostData.subPID = 0;
                        }
                    }
                    else if (endedTerminal)
                    {
                        using (new FilesystemLock(childComputerId, Interpreter.GetScriptID()))
                        {
                            Computer remoteComputer = Interpreter.hostData.GetRemoteComputer(
                                childComputerId,
                                partialObj: false,
                                checkConnection: false
                            );
                            if (remoteComputer != null)
                            {
                                Interpreter.hostData.CloseRemotePrograms(
                                    remoteComputer,
                                    bouncesBeforeExit
                                );
                            }
                        }
                    }
                }
            } while (Interpreter.Running());

            process?.RunRequests();

            Interpreter.SendPendingPrint();
            Interpreter
                .GetHostComputer()
                ?.CloseProgram(Interpreter.hostData.PID, Interpreter, scriptFinished: true);

            MessageProcess messageProcess = new MessageProcess(
                IdProcess.ScriptEnd,
                Interpreter.hostData
            );
            Interpreter.SendMessage(messageProcess);
            // End the context only after the client receives the program's final print.
            (launcher as AppController)?.MessagesEnd.Wait(ListenTime);
            process?.Finish(Interpreter.hostData.scriptCrash);
        }
    }
}
