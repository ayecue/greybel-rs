using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Sockets;
using BepInEx;
using GreyHackMessageHook.Game;
using HarmonyLib;
using NetworkMessages;
using Newtonsoft.Json;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    [HarmonyPatch(typeof(PlayerClientMethods))]
    public static class PlayerClientMethodsPatch
    {
        [HarmonyPrefix]
        [HarmonyPatch("SendCommandClientRpc")]
        static bool SendCommandClientRpcPrefix(
            PlayerClientMethods __instance,
            int terminalPID,
            byte[] zipFile,
            string pathFile,
            int PID,
            byte[] zipProcs,
            string deviceName,
            string activeUser,
            bool isRemoteConnection,
            string remoteNetID,
            byte[] zipArguments
        )
        {
            var process = ContextManager.GetContextByPid(terminalPID);

            if (process != null)
            {
                string line = (
                    (zipFile == null || zipFile.Length == 0) ? "" : StringCompressor.Unzip(zipFile)
                );
                if (!string.IsNullOrEmpty(line))
                {
                    List<string> arguments = new List<string>();
                    if (zipArguments != null)
                    {
                        arguments = JsonConvert.DeserializeObject<List<string>>(
                            StringCompressor.Unzip(zipArguments)
                        );
                    }
                    FileSystem.Archivo file = JsonConvert.DeserializeObject<FileSystem.Archivo>(
                        line
                    );
                    file.SetForcePath(pathFile);
                    ThreadingHelper.Instance.StartSyncInvoke(() =>
                    {
                        InternalBash.Singleton.ResumeCommand(
                            file,
                            PID,
                            deviceName,
                            activeUser,
                            isRemoteConnection,
                            arguments,
                            remoteNetID
                        );
                        PlayerUtils.UpdateWidgets(zipProcs);
                    });
                }
                return false;
            }

            return true;
        }
    }

    public static class PlayerClientEx
    {
        public static void ReceiveData(PlayerClient playerClient)
        {
            TcpClient tcpClient = (TcpClient)
                RelevantGameMemberTable.PlayerClientClient.GetValue(playerClient);

            try
            {
                Stream stream = tcpClient.GetStream();
                RelevantGameMemberTable.PlayerClientStream.SetValue(playerClient, stream);
                string line;
                while ((line = RpcFrame.Read(stream, RpcFrame.MaxPayloadBytes)) != null)
                {
                    try
                    {
                        Logger.Log("PlayerClientEx - Received message: " + line);
                        ProcessReceivedData(RawMessage.Parse(line), playerClient);
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
                    Logger.Log("Client: Disconnected from server");
                }
                else
                {
                    Logger.LogError("A SocketException occurred: " + ex2.SocketErrorCode);
                }
            }
            finally
            {
                playerClient.Disconnect();
            }
        }

        public static void SendData(RawMessage message, PlayerClient playerClient)
        {
            Stream stream = (Stream)
                RelevantGameMemberTable.PlayerClientStream.GetValue(playerClient);

            try
            {
                Logger.Log("PlayerClientEx - SendData");
                byte[] bytes = RpcFrame.Serialize(message.ToObject<Newtonsoft.Json.Linq.JObject>());
                stream.Write(bytes, 0, bytes.Length);
            }
            catch (ArgumentNullException ex)
            {
                Logger.LogError("ArgumentNullException: " + ex);
            }
            catch (SocketException ex2)
            {
                Logger.LogError("SocketException: " + ex2);
            }
        }

        public static void ProcessReceivedData(RawMessage raw, PlayerClient playerClient)
        {
            if (!Proxy.Converter.IsExtension(raw, Duplex.Role.Provider))
            {
                MessageClient gameMessage = raw.ToGameMessage<MessageClient>();
                ThreadingHelper.Instance.StartSyncInvoke(() =>
                {
                    RelevantGameMemberTable.PlayerClientProcessReceivedData.Invoke(
                        playerClient,
                        new object[] { gameMessage }
                    );
                });
            }

            Proxy.ProcessReceivedData(raw);
        }
    }
}
