using System.Collections.Generic;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using BepInEx;
using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;
using HarmonyLib;
using NetworkMessages;
using Newtonsoft.Json;
using ProgramVisual;
using Message = Duplex.Message;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    [HarmonyPatch(typeof(ProgramWindow))]
    public static class ProgramWindowProcesarPatch
    {
        [HarmonyPostfix]
        [HarmonyPatch("Procesar")]
        static void ProcesarPostfix(ProgramWindow __instance)
        {
            Ventana window = (Ventana)
                RelevantGameMemberTable.ProgramWindowVentana.GetValue(__instance);
            WindowHandler.EmitNewWindow(window);
        }
    }

    public delegate void NewWindowEventHandler(Ventana window);
    public delegate void WindowCloseEventHandler(WindowHandler handler);
    public delegate void WindowInputReadyEventHandler(WindowHandler handler);

    public class WindowHandler
    {
        private const int DefaultWaitMs = 1000;
        private const int RequestTimeoutMs = 10000;
        public static event NewWindowEventHandler NewWindow;

        public static void EmitNewWindow(Ventana window)
        {
            NewWindow?.Invoke(window);
        }

        public static void AddNewWindowListener(NewWindowEventHandler handler)
        {
            NewWindow += handler;
        }

        public static void RemoveNewWindowListener(NewWindowEventHandler handler)
        {
            NewWindow -= handler;
        }

        private static bool IsDeliveredTo(Message message, int pid)
        {
            return Proxy.Converter.GetWindowPids(message).Contains(pid);
        }

        private static Task<Ventana> WaitForWindow(int pid, int timeoutMs = DefaultWaitMs)
        {
            TaskCompletionSource<Ventana> tcs = new TaskCompletionSource<Ventana>();
            NewWindowEventHandler handler = null;
            System.Threading.CancellationTokenSource cts =
                new System.Threading.CancellationTokenSource();

            handler = (Ventana window) =>
            {
                if (window.GetPID() != pid)
                    return;
                cts.Cancel();
                tcs.SetResult(window);
                RemoveNewWindowListener(handler);
            };

            AddNewWindowListener(handler);

            Task.Delay(timeoutMs, cts.Token)
                .ContinueWith(
                    t =>
                    {
                        if (!t.IsCanceled && !tcs.Task.IsCompleted)
                        {
                            RemoveNewWindowListener(handler);
                            tcs.SetException(
                                new System.TimeoutException(
                                    $"Timeout waiting for window with PID {pid} after {timeoutMs}ms"
                                )
                            );
                        }
                    },
                    TaskContinuationOptions.ExecuteSynchronously
                );

            return tcs.Task;
        }

        public event WindowCloseEventHandler CloseEvent;
        public event WindowInputReadyEventHandler InputReady;

        private PlayerClient client;
        private Ventana window;
        private bool inputReady = false;
        private string outputBuffer = "";
        private bool disposed = false;
        private bool closed = false;

        public static Task<WindowHandler> OpenWindow(
            PlayerClient client,
            string command,
            string user = ContextManager.RootUser
        )
        {
            TaskCompletionSource<WindowHandler> tcs = new TaskCompletionSource<WindowHandler>();

            Task.Run(async () =>
            {
                try
                {
                    FileSystem.Carpeta folder = client
                        .player.pc.GetFileSystem()
                        .GetCarpetaUser(user);
                    string remoteNetId = client.player.pc.GetID();
                    InternalBash.Singleton.ProcesaLinea(command, remoteNetId, folder, user);

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            return (
                                    candidate is SendCommandClientRpc
                                    || candidate is CloseConnectionClientRpc
                                    || candidate is SendOutputClientRpc
                                    || candidate is FailedDiskClientRpc
                                    || candidate is FailedProcessClientRpc
                                    || candidate is CloseProgramClientRpc
                                ) && IsDeliveredTo(candidate, InternalBash.Singleton.GetPID());
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for open window candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to open command: " + closeConnection.Message
                        );
                    }

                    if (!(candidate is SendCommandClientRpc sendCommandClientRpc))
                    {
                        throw new System.Exception("Unexpected candidate when opening command");
                    }

                    Ventana window = WaitForWindow(
                        sendCommandClientRpc.Pid,
                        RequestTimeoutMs
                    ).Result;

                    if (window == null)
                    {
                        throw new System.Exception("Failed to open window for command: " + command);
                    }

                    tcs.SetResult(new WindowHandler(client, window));
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public WindowHandler(PlayerClient client, Ventana window)
        {
            this.client = client;
            this.window = window;

            InitializeListeners();
        }

        private void InitializeListeners()
        {
            Proxy.RegisterMessageListener(ReceiveMessage);
        }

        private void ReceiveMessage(Message candidate)
        {
            if (!IsDeliveredTo(candidate, window.GetPID()))
                return;
            switch (candidate)
            {
                case InputSentClientRpc _:
                {
                    Logger.Log("InputSentClientRpc received, marking input as ready");
                    inputReady = true;
                    InputReady?.Invoke(this);
                    break;
                }
                case PrintSentClientRpc printMessage:
                {
                    Logger.Log("PrintSentClientRpc received, appending output");
                    outputBuffer += StringCompressor.Unzip(printMessage.ZipOutput);
                    break;
                }
                case CloseProgramClientRpc _:
                {
                    Logger.Log("CloseProgramClientRpc received, marking as disposed");
                    closed = true;
                    CloseEvent?.Invoke(this);

                    if (!disposed)
                    {
                        Dispose();
                    }

                    break;
                }
            }
        }

        public Task<FileSystem.Carpeta> GetFolder(string path)
        {
            if (disposed)
                return Task.FromException<FileSystem.Carpeta>(
                    new System.Exception("WindowHandler is disposed")
                );
            if (closed)
                return Task.FromException<FileSystem.Carpeta>(
                    new System.Exception("WindowHandler is closed")
                );

            TaskCompletionSource<FileSystem.Carpeta> tcs =
                new TaskCompletionSource<FileSystem.Carpeta>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    FolderContentsServerRpc request = new FolderContentsServerRpc(
                        path,
                        window.GetActiveUser(),
                        window.GetPID()
                    );
                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            return (
                                    candidate is CloseConnectionClientRpc
                                    || candidate is UpdateWindowClientRpc
                                ) && IsDeliveredTo(candidate, window.GetPID());
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for folder contents candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to get folder content: " + closeConnection.Message
                        );
                    }

                    UpdateWindowClientRpc updateWindow = (UpdateWindowClientRpc)candidate;
                    FileSystem.Carpeta folder = JsonConvert.DeserializeObject<FileSystem.Carpeta>(
                        StringCompressor.Unzip(updateWindow.ZipFolder)
                    );

                    tcs.SetResult(folder);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public Task<bool> MoveFile(
            string sourcePath,
            string destinationDirectory,
            string destinationFileName
        )
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    CheckMoveFileServerRpc request = new CheckMoveFileServerRpc(
                        destinationDirectory,
                        sourcePath,
                        destinationFileName,
                        window.GetRemoteNetID(),
                        false,
                        true,
                        window.GetActiveUser(),
                        window.GetPID()
                    );
                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            if (candidate is SendPendingFoldersClientRpc pendingFolders)
                            {
                                if (
                                    pendingFolders.ZipEndCopy == null
                                    || pendingFolders.ZipEndCopy.Length == 0
                                )
                                    return true;

                                List<HelperScript.HelperCopyEnd> list =
                                    JsonConvert.DeserializeObject<List<HelperScript.HelperCopyEnd>>(
                                        StringCompressor.Unzip(pendingFolders.ZipEndCopy)
                                    );
                                HelperScript.HelperCopyEnd associatedEntry = list.Find(
                                    (item) => item != null && item.windowPID == window.GetPID()
                                );
                                return associatedEntry == null;
                            }

                            return (
                                    candidate is CheckInicioClientRpc
                                    || candidate is FailedDiskClientRpc
                                    || candidate is CloseConnectionClientRpc
                                ) && IsDeliveredTo(candidate, window.GetPID());
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for move file candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to move file: " + closeConnection.Message
                        );
                    }

                    if (!(candidate is SendPendingFoldersClientRpc))
                    {
                        throw new System.Exception("Unexpected candidate when moving file");
                    }

                    tcs.SetResult(true);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public Task<bool> CreateFolder(string parent, string name)
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    CrearCarpetaServerRpc request = new CrearCarpetaServerRpc(
                        parent,
                        name,
                        window.GetActiveUser(),
                        window.GetPID()
                    );
                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            return candidate is SendPendingFoldersClientRpc
                                || (
                                    candidate is CloseConnectionClientRpc
                                    && IsDeliveredTo(candidate, window.GetPID())
                                );
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for create folder candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to create folder "
                                + PosixPath.Join(parent, name)
                                + ": "
                                + closeConnection.Message
                        );
                    }

                    tcs.SetResult(true);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public Task<bool> AddResource(string content, string path)
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    ModifyFileServerRpc request = new ModifyFileServerRpc(
                        PosixPath.GetDirectoryName(path),
                        PosixPath.GetFileName(path),
                        StringCompressor.Zip(content),
                        3,
                        window.GetPID()
                    );
                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            return (
                                    candidate is ShowNotificationClientRpc
                                    || candidate is SuccessWindowClientRpc
                                    || candidate is CloseConnectionClientRpc
                                ) && IsDeliveredTo(candidate, window.GetPID());
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for add resource candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to add resource: " + closeConnection.Message
                        );
                    }

                    tcs.SetResult(true);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public Task<string> BuildScript(
            string scriptContent,
            string fileName,
            string path = "/",
            string user = ContextManager.RootUser,
            bool isImportable = false
        )
        {
            if (disposed)
                return Task.FromException<string>(
                    new System.Exception("WindowHandler is disposed")
                );
            if (closed)
                return Task.FromException<string>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<string> tcs = new TaskCompletionSource<string>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    BuildScriptServerRpc request = new BuildScriptServerRpc(
                        path,
                        fileName,
                        StringCompressor.Zip(scriptContent),
                        isImportable,
                        user,
                        window.GetPID()
                    );
                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());

                    Message candidate = await Proxy.WaitForReceivedMessage(
                        (candidate) =>
                        {
                            return (
                                    candidate is ShowNotificationClientRpc
                                    || candidate is CloseConnectionClientRpc
                                ) && IsDeliveredTo(candidate, window.GetPID());
                        },
                        RequestTimeoutMs
                    );

                    if (candidate == null)
                    {
                        throw new System.Exception("Timeout waiting for build script candidate");
                    }

                    if (candidate is CloseConnectionClientRpc closeConnection)
                    {
                        throw new System.Exception(
                            "Failed to build script: " + closeConnection.Message
                        );
                    }

                    ShowNotificationClientRpc notification = (ShowNotificationClientRpc)candidate;

                    tcs.SetResult(notification.Message);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public Task<bool> IsTerminalReady()
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            if (!(window is Terminal))
            {
                tcs.SetException(new System.Exception("Window is not a terminal"));
                return tcs.Task;
            }

            if (inputReady)
            {
                tcs.SetResult(true);
                return tcs.Task;
            }

            Terminal terminalWindow = (Terminal)window;
            WindowCloseEventHandler closeHandler = null;
            WindowInputReadyEventHandler inputReadyHandler = null;
            closeHandler = (WindowHandler handler) =>
            {
                CloseEvent -= closeHandler;
                InputReady -= inputReadyHandler;
                tcs.SetException(
                    new System.Exception("Window was closed before terminal became ready")
                );
            };
            inputReadyHandler = (WindowHandler handler) =>
            {
                InputReady -= inputReadyHandler;
                CloseEvent -= closeHandler;
                tcs.SetResult(true);
            };

            CloseEvent += closeHandler;
            InputReady += inputReadyHandler;

            return tcs.Task;
        }

        public Task<bool> IsTerminalReadyWithTimeout(int timeoutMs = DefaultWaitMs)
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is closed"));

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            if (!(window is Terminal))
            {
                tcs.SetException(new System.Exception("Window is not a terminal"));
                return tcs.Task;
            }

            if (inputReady)
            {
                tcs.SetResult(true);
                return tcs.Task;
            }

            System.Threading.CancellationTokenSource cts =
                new System.Threading.CancellationTokenSource();

            Terminal terminalWindow = (Terminal)window;
            WindowCloseEventHandler closeHandler = null;
            WindowInputReadyEventHandler inputReadyHandler = null;
            closeHandler = (WindowHandler handler) =>
            {
                cts.Cancel();
                CloseEvent -= closeHandler;
                InputReady -= inputReadyHandler;
                tcs.SetException(
                    new System.Exception("Window was closed before terminal became ready")
                );
            };
            inputReadyHandler = (WindowHandler handler) =>
            {
                cts.Cancel();
                InputReady -= inputReadyHandler;
                CloseEvent -= closeHandler;
                tcs.SetResult(true);
            };

            CloseEvent += closeHandler;
            InputReady += inputReadyHandler;

            Task.Delay(timeoutMs, cts.Token)
                .ContinueWith(
                    t =>
                    {
                        if (!t.IsCanceled && !tcs.Task.IsCompleted)
                        {
                            CloseEvent -= closeHandler;
                            InputReady -= inputReadyHandler;
                            tcs.SetException(
                                new System.TimeoutException(
                                    $"Timeout waiting for terminal to be ready after {timeoutMs}ms"
                                )
                            );
                        }
                    },
                    TaskContinuationOptions.ExecuteSynchronously
                );

            return tcs.Task;
        }

        public Task<ScriptExecutionResult> RunScriptInTerminal(string command)
        {
            if (disposed)
                return Task.FromException<ScriptExecutionResult>(
                    new System.Exception("WindowHandler is disposed")
                );
            if (closed)
                return Task.FromException<ScriptExecutionResult>(
                    new System.Exception("WindowHandler is closed")
                );

            TaskCompletionSource<ScriptExecutionResult> tcs =
                new TaskCompletionSource<ScriptExecutionResult>();

            ThreadingHelper.Instance.StartSyncInvoke(async () =>
            {
                try
                {
                    Logger.Log("Waiting for terminal to be ready...");
                    bool isBashReady = await IsTerminalReadyWithTimeout(RequestTimeoutMs);

                    if (!isBashReady)
                    {
                        throw new System.Exception("Terminal bash is not ready");
                    }

                    inputReady = false;
                    InputUserServerRpc inputUserRequest = new InputUserServerRpc(
                        StringCompressor.Zip(command),
                        window.GetPID()
                    );
                    client.SendData(
                        Proxy.Converter.Encode(inputUserRequest).ToGameMessage<MessageServer>()
                    );

                    Logger.Log($"Sent command to terminal: {command}");

                    await IsTerminalReady();

                    Logger.Log("Command completed, parsing output for exit codes");

                    ScriptExecutionResult result = ParseScriptOutput(outputBuffer);
                    outputBuffer = "";

                    tcs.SetResult(result);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        private ScriptExecutionResult ParseScriptOutput(string output)
        {
            ScriptExecutionResult result = new ScriptExecutionResult { Output = output };

            string pattern = @"EXIT_CODE=(\d+);(?:EXIT_MESSAGE=([^;]*);)?$";
            Match match = Regex.Match(output, pattern);

            if (match.Success)
            {
                if (int.TryParse(match.Groups[1].Value, out int exitCode))
                {
                    result.ExitCode = exitCode;
                }

                if (match.Groups[2].Success && !string.IsNullOrEmpty(match.Groups[2].Value))
                {
                    result.ExitMessage = match.Groups[2].Value;
                }
            }

            return result;
        }

        public async Task<ScriptExecutionResult> RunTemporaryScriptInTerminal(string scriptContent)
        {
            if (disposed)
                throw new System.Exception("WindowHandler is disposed");
            if (closed)
                throw new System.Exception("WindowHandler is closed");

            string fileName = BuildHelper.GenerateRandomString(BuildHelper.TemporaryFileNameLength);
            string path = PosixPath.Join("/", fileName);

            Logger.Log($"Creating temporary script at {path}");
            await BuildScript(
                $"get_shell.host_computer.File(\"{path}\").delete;{scriptContent}",
                fileName
            );

            Thread.Sleep(DefaultWaitMs);

            Logger.Log("Running temporary script in terminal");
            ScriptExecutionResult result = await RunScriptInTerminal(path);

            Logger.Log("Temporary script executed and deleted");

            return result;
        }

        public Task<bool> Close()
        {
            if (disposed)
                return Task.FromException<bool>(new System.Exception("WindowHandler is disposed"));
            if (closed)
                return Task.FromResult(true);

            TaskCompletionSource<bool> tcs = new TaskCompletionSource<bool>();

            ThreadingHelper.Instance.StartSyncInvoke(() =>
            {
                try
                {
                    window.CloseTaskBar();
                    tcs.SetResult(true);
                }
                catch (System.Exception e)
                {
                    tcs.SetException(e);
                }
            });

            return tcs.Task;
        }

        public async Task Dispose()
        {
            if (disposed)
                return;
            disposed = true;
            await Close();
            Proxy.UnregisterMessageListener(ReceiveMessage);
        }
    }

    public class ScriptExecutionResult
    {
        public int ExitCode { get; set; } = 0;
        public string ExitMessage { get; set; } = "";
        public string Output { get; set; } = "";
    }
}
