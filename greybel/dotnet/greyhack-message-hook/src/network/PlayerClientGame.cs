using System;
using System.Threading.Tasks;
using BepInEx;
using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;
using NetworkMessages;
using Newtonsoft.Json;
using Message = Duplex.Message;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    public sealed class PlayerClientGame : IGame
    {
        private const int InternalBashPid = -10;
        private static PlayerClient Current => Proxy.GetPlayerClient();

        private static class MessageHandler
        {
            public static void HandlePrepareCommandServerRpc(
                PrepareCommandServerRpc request,
                PlayerClient client
            )
            {
                string[] arguments = JsonConvert.DeserializeObject<string[]>(
                    StringCompressor.Unzip(request.ZipComandoCompleto)
                );

                if (request.TerminalPid == InternalBashPid)
                {
                    ThreadingHelper.Instance.StartSyncInvoke(() =>
                    {
                        string line = String.Join(" ", arguments);
                        FileSystem.Carpeta folder = client
                            .player.pc.GetFileSystem()
                            .GetCarpetaUser(request.PropActiveUser);
                        string remoteNetId = client.player.pc.GetID();
                        Logger.Log("Executing command \"" + line + "\" on internal bash");
                        InternalBash.Singleton.ProcesaLinea(
                            line,
                            remoteNetId,
                            folder,
                            request.PropActiveUser
                        );
                    });
                    return;
                }

                ThreadingHelper.Instance.StartSyncInvoke(() =>
                {
                    Ventana window = client.player.GetVentana(request.TerminalPid);

                    if (window == null)
                    {
                        Logger.Log("Unable to find window with PID " + request.TerminalPid);
                        Proxy.Publish(
                            new CloseConnectionClientRpc(
                                request.TerminalPid,
                                "Window is not available!",
                                false
                            )
                        );
                        return;
                    }

                    if (window is Terminal)
                    {
                        Terminal terminal = (Terminal)window;
                        string line = String.Join(" ", arguments);

                        Logger.Log(
                            "Executing command \""
                                + line
                                + "\" on terminal with PID "
                                + request.TerminalPid
                        );
                        terminal.PrintLinea(line);
                    }

                    client.SendData(Proxy.Converter.Encode(request).ToGameMessage<MessageServer>());
                });
            }

            public static void HandleCloseProgramServerRpc(
                CloseProgramServerRpc request,
                PlayerClient client
            )
            {
                ThreadingHelper.Instance.StartSyncInvoke(() =>
                {
                    Ventana window = client.player.GetVentana(request.Pid);

                    if (window == null)
                    {
                        Proxy.Publish(
                            new CloseConnectionClientRpc(
                                InternalBashPid,
                                "Window is not available!",
                                false
                            )
                        );
                        return;
                    }

                    window.CloseTaskBar();
                    client.player.pc.CloseProgramClient(request.Pid);
                });
            }
        }

        private static void Dispatch(Message message)
        {
            switch (message)
            {
                case UserLoginServerRpc _:
                    Proxy.Publish(ResponseFactory.CreatePlayerLoad(Current));
                    break;
                case PrepareCommandServerRpc request:
                    MessageHandler.HandlePrepareCommandServerRpc(request, Current);
                    break;
                case CloseProgramServerRpc request:
                    MessageHandler.HandleCloseProgramServerRpc(request, Current);
                    break;
                case HealthCheckRpc _:
                    Proxy.Publish(ResponseFactory.CreateStatus());
                    break;
                case CreateBuildRpc request:
                    BuildManager.CreateBuild(
                        request.BuildId,
                        request.Destination,
                        request.ResourceDestination,
                        request.AutoCompile,
                        Current
                    );
                    Proxy.Publish(new CreatedBuildRpc(request.BuildId));
                    break;
                case AddResourceToBuildRpc request:
                {
                    BuildManager.BuildProcess process = BuildManager.GetBuild(request.BuildId);

                    if (process == null)
                    {
                        Proxy.Publish(new InvalidAction());
                        break;
                    }

                    process.AddResource(
                        request.Location,
                        StringCompressor.Unzip(request.ZipContent)
                    );
                    Proxy.Publish(new AddedResourceToBuildRpc(request.BuildId));
                    break;
                }
                case AddScriptToBuildRpc request:
                {
                    BuildManager.BuildProcess process = BuildManager.GetBuild(request.BuildId);

                    if (process == null)
                    {
                        Proxy.Publish(new InvalidAction());
                        break;
                    }

                    process.AddScript(
                        (BuildManager.ScriptType)request.Type,
                        StringCompressor.Unzip(request.ZipContent)
                    );
                    Proxy.Publish(new AddedScriptToBuildRpc(request.BuildId));
                    break;
                }
                case BuildStateServerRpc request:
                {
                    BuildManager.BuildProcess process = BuildManager.GetBuild(request.BuildId);

                    if (process == null)
                    {
                        Proxy.Publish(
                            ResponseFactory.CreateBuildState(
                                "",
                                BuildManager.BuildState.Unknown,
                                "Build ID not found"
                            )
                        );
                        break;
                    }

                    Proxy.Publish(
                        ResponseFactory.CreateBuildState(
                            request.BuildId,
                            process.GetState(),
                            process.GetErrorMessage(),
                            process.GetOutput()
                        )
                    );
                    break;
                }
                case PerformBuildRpc request:
                {
                    BuildManager.BuildProcess process = BuildManager.GetBuild(request.BuildId);

                    if (process == null)
                    {
                        Proxy.Publish(new InvalidAction());
                        break;
                    }

                    Task.Run(async () =>
                    {
                        await process.Run();
                        Proxy.Publish(
                            ResponseFactory.CreateBuildCompletion(
                                request.BuildId,
                                process.GetState(),
                                process.GetErrorMessage(),
                                process.GetOutput()
                            )
                        );
                    });
                    break;
                }
                case DisposeBuildRpc request:
                {
                    BuildManager.BuildProcess process = BuildManager.GetBuild(request.BuildId);

                    if (process == null)
                    {
                        Proxy.Publish(new InvalidAction());
                        break;
                    }

                    bool disposed = BuildManager.DisposeBuild(request.BuildId);
                    Proxy.Publish(
                        ResponseFactory.CreateDisposedBuild(
                            request.BuildId,
                            disposed,
                            process.GetState(),
                            process.GetErrorMessage(),
                            process.GetOutput()
                        )
                    );
                    break;
                }
                case CreateContextRpc _:
                case ContextRuntimeStateServerRpc _:
                case ContextDebugModeRpc _:
                case ContextStepRpc _:
                case ContextPassBreakpointRpc _:
                case ContextInjectCodeRpc _:
                case ContextSetBreakpointsRpc _:
                case ContextEvaluateRpc _:
                case DisposeContextRpc _:

                    PlayerClientEx.SendData(Proxy.Converter.Encode(message), Current);
                    break;
                default:
                    if (Proxy.Converter.IsExtension(message))
                    {
                        Proxy.Publish(new InvalidAction());
                        break;
                    }
                    Current.SendData(
                        Proxy.Converter.Encode(message).ToGameMessage<MessageServer>()
                    );
                    break;
            }
        }

        public void Send(Message message)
        {
            if (Current == null)
            {
                if (message is HealthCheckRpc)
                {
                    Proxy.Publish(ResponseFactory.CreateStatus());
                    return;
                }

                Logger.LogError("PlayerClient is not defined");
                return;
            }

            try
            {
                Dispatch(message);
            }
            catch (Exception ex)
            {
                Logger.LogError(ex);
                Logger.Log(ex);
            }
        }
    }
}
