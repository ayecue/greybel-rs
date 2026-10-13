using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using BepInEx;
using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Greybel;
using GreyHackMessageHook.Protocol;
using HarmonyLib;
using Miniscript;
using NetworkMessages;
using Newtonsoft.Json;
using Message = Duplex.Message;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    public static class PlayerServerEx
    {
        private const int StepInto = 0;
        private const int StepOver = 1;
        private const int StepOut = 2;
        public static PlayerServer Current { get; private set; }

        public static void ReceiveData(PlayerServer server)
        {
            Current = server;
            Stream stream = (Stream)RelevantGameMemberTable.PlayerServerStream.GetValue(server);
            try
            {
                string line;
                while ((line = RpcFrame.Read(stream, RpcFrame.MaxPayloadBytes)) != null)
                {
                    try
                    {
                        Logger.Log("PlayerServerEx - Received message: " + line);
                        RawMessage message = RawMessage.Parse(line);
                        ThreadingHelper.Instance.StartAsyncInvoke(() =>
                        {
                            ProcessReceivedData(message, server);
                            return null;
                        });
                    }
                    catch (Exception error)
                    {
                        Logger.LogError(error);
                    }
                }
            }
            catch (IOException ex)
            {
                if (ex.InnerException == null)
                {
                    Logger.LogError(ex);
                }
                else if (!(ex.InnerException is SocketException ex2))
                {
                    Logger.LogError(ex.InnerException);
                }
                else if (ex2.SocketErrorCode == SocketError.Interrupted)
                {
                    Logger.Log("Server: client connection interrupted");
                }
                else
                {
                    Logger.LogError("A SocketException occurred: " + ex2.SocketErrorCode);
                }
            }
            finally
            {
                server?.OnDisconnect();
                Logger.Log("Server: client connection closed");
            }
        }

        public static void SendData(RawMessage message, PlayerServer server)
        {
            object lockWrite = RelevantGameMemberTable.PlayerServerLockWrite.GetValue(server);
            Stream stream = (Stream)RelevantGameMemberTable.PlayerServerStream.GetValue(server);
            byte[] bytes = RpcFrame.Serialize(message.ToObject<Newtonsoft.Json.Linq.JObject>());

            lock (lockWrite)
            {
                if ((bool)RelevantGameMemberTable.PlayerServerSocketClose.GetValue(server))
                {
                    return;
                }
                try
                {
                    Logger.Log("PlayerServerEx - SendData");
                    stream.Write(bytes, 0, bytes.Length);
                }
                catch (IOException ex)
                {
                    if (ex.InnerException == null)
                    {
                        Logger.LogError(ex);
                        return;
                    }
                    if (!(ex.InnerException is SocketException ex2))
                    {
                        Logger.LogError(ex.InnerException);
                        return;
                    }
                    if (ex2.SocketErrorCode == SocketError.Interrupted)
                    {
                        Logger.Log("SendData: client connection interrupted");
                    }
                    else
                    {
                        Logger.LogError(
                            "SendData: A SocketException occurred: " + ex2.SocketErrorCode
                        );
                    }
                    RelevantGameMemberTable.PlayerServerSocketClose.SetValue(server, true);
                    server.greyScriptHelper.KillAllScripts();
                }
            }
        }

        public static void SendData(Message message, PlayerServer server)
        {
            SendData(Proxy.Converter.Encode(message), server);
        }

        public static void ProcessReceivedData(RawMessage raw, PlayerServer server)
        {
            if (!Proxy.Converter.IsExtension(raw, Role.Client))
            {
                RelevantGameMemberTable.PlayerServerProcessReceivedData.Invoke(
                    server,
                    new object[] { raw.ToGameMessage<MessageServer>() }
                );
                return;
            }

            Logger.Log("PlayerServerEx - ProcessReceivedData: " + raw);

            try
            {
                switch (Proxy.Converter.Decode(raw, Role.Client))
                {
                    case CreateContextRpc request:
                    {
                        string source = StringCompressor.Unzip(request.Source);
                        List<RuntimeMetadata.Breakpoint> breakpoints =
                            JsonConvert.DeserializeObject<List<RuntimeMetadata.Breakpoint>>(
                                StringCompressor.Unzip(request.Breakpoints)
                            );
                        List<string> parameters =
                            JsonConvert.DeserializeObject<List<string>>(
                                StringCompressor.Unzip(request.Params)
                            ) ?? new List<string>();
                        SourceMap sourceMap = SourceMap.FromJson(
                            StringCompressor.Unzip(request.SourceMap)
                        );
                        PlayerComputer computer = server.GetComputer();
                        ContextManager.CreateContext(
                            request.ContextId,
                            request.DebugMode,
                            server,
                            request.Basepath,
                            request.Filepath,
                            request.ProgramName,
                            source,
                            sourceMap,
                            request.StrictMode,
                            parameters,
                            computer,
                            breakpoints
                        );
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );
                        SendData(new CreatedContextRpc(request.ContextId, process.Pid), server);

                        break;
                    }
                    case ContextRuntimeStateServerRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            process.Post(() =>
                            {
                                var context =
                                    process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                                var metadata = RuntimeMetadataManager.GetMetadata(
                                    context.interpreter
                                );
                                var stackTrace = metadata.GetStackTrace();

                                SendData(
                                    ResponseFactory.CreateContextRuntimeState(
                                        request.ContextId,
                                        new List<ValMap>
                                        {
                                            context.root.variables,
                                            context.outerVars,
                                            context.variables,
                                        },
                                        stackTrace
                                    ),
                                    server
                                );
                            });
                        }

                        break;
                    }
                    case ContextDebugModeRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            process.Post(() =>
                            {
                                var context =
                                    process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                                var metadata = RuntimeMetadataManager.GetMetadata(
                                    context.interpreter
                                );
                                bool shouldResume = metadata.activeBreakpoint;

                                metadata.activeBreakpoint = request.ActiveBreakpoint;
                                if (!request.ActiveBreakpoint)
                                {
                                    metadata.stopDepth = null;
                                }

                                if (shouldResume)
                                {
                                    process.AppController.AppInstance.Interpreter.Resume();
                                }
                            });
                        }

                        break;
                    }
                    case ContextInjectCodeRpc request:
                    {
                        string source = StringCompressor.Unzip(request.Source);
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            process.Post(() =>
                            {
                                var context =
                                    process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                                var metadata = RuntimeMetadataManager.GetMetadata(
                                    context.interpreter
                                );

                                ContextHelper.InjectCode(context, source);

                                context.lineNum--;
                                process.AppController.AppInstance.Interpreter.Resume();
                            });
                        }

                        break;
                    }
                    case ContextStepRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            process.Post(() =>
                            {
                                var context =
                                    process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                                var metadata = RuntimeMetadataManager.GetMetadata(
                                    context.interpreter
                                );

                                if (metadata.activeBreakpoint && metadata.pendingEvaluation == null)
                                {
                                    int depth = ContextHelper.GetStack(context).Count;
                                    metadata.activeBreakpoint = false;

                                    metadata.stopDepth =
                                        request.Mode == StepOver ? depth
                                        : request.Mode == StepOut ? depth - 1
                                        : int.MaxValue;
                                    process.AppController.AppInstance.Interpreter.Resume();
                                }
                            });
                        }

                        break;
                    }
                    case ContextPassBreakpointRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            process.Post(() =>
                            {
                                var context =
                                    process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                                var metadata = RuntimeMetadataManager.GetMetadata(
                                    context.interpreter
                                );

                                if (metadata.activeBreakpoint && metadata.pendingEvaluation == null)
                                {
                                    metadata.activeBreakpoint = false;
                                    process.AppController.AppInstance.Interpreter.Resume();
                                }
                            });
                        }

                        break;
                    }
                    case ContextSetBreakpointsRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process != null)
                        {
                            List<RuntimeMetadata.Breakpoint> breakpoints =
                                JsonConvert.DeserializeObject<List<RuntimeMetadata.Breakpoint>>(
                                    StringCompressor.Unzip(request.Breakpoints)
                                ) ?? new List<RuntimeMetadata.Breakpoint>();

                            process.Post(() => process.Breakpoints = breakpoints);
                        }

                        break;
                    }
                    case ContextEvaluateRpc request:
                    {
                        ContextManager.ContextProcess process = ContextManager.GetContext(
                            request.ContextId
                        );

                        if (process == null)
                        {
                            break;
                        }

                        process.Post(() =>
                        {
                            var context =
                                process.AppController.AppInstance.Interpreter.vm.GetTopContext();
                            var metadata = RuntimeMetadataManager.GetMetadata(context.interpreter);

                            if (!metadata.activeBreakpoint || metadata.pendingEvaluation != null)
                            {
                                SendData(
                                    new ContextEvaluatedRpc(
                                        request.ContextId,
                                        true,
                                        "The program runs: code is evaluated where it stopped.",
                                        false,
                                        new byte[0]
                                    ),
                                    server
                                );
                                return;
                            }

                            string error = metadata.StartEvaluation(
                                context,
                                StringCompressor.Unzip(request.Code)
                            );

                            if (error != null)
                            {
                                SendData(
                                    new ContextEvaluatedRpc(
                                        request.ContextId,
                                        true,
                                        error,
                                        false,
                                        new byte[0]
                                    ),
                                    server
                                );
                                return;
                            }

                            process.AppController.AppInstance.Interpreter.Resume();
                        });
                        break;
                    }
                    case DisposeContextRpc request:
                    {
                        if (ContextManager.DisposeContext(request.ContextId))
                        {
                            SendData(new DisposedContextRpc(request.ContextId), server);
                        }

                        break;
                    }
                }
            }
            catch (Exception ex)
            {
                Logger.LogError(ex);
            }
        }
    }
}
