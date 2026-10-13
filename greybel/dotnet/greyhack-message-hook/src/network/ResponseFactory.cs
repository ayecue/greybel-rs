using System.Collections.Generic;
using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;
using Miniscript;
using Newtonsoft.Json;
using Util;
using static GreyHackMessageHook.Greybel.RuntimeMetadata;
using Roles = GreyHackMessageHook.Protocol.Roles;

namespace GreyHackMessageHook
{
    public static class ResponseFactory
    {
        public static Message CreatePlayerLoad(PlayerClient playerClient)
        {
            return new PlayerLoadClientRpc(
                Roles.Everyone,
                new byte[0],
                StringCompressor.Zip(
                    JsonConvert.SerializeObject(playerClient.player.pc.GetUsers())
                ),
                StringCompressor.Zip(playerClient.player.pc.GetConfigOS().SerializeObject()),
                StringCompressor.Zip(
                    JsonConvert.SerializeObject(playerClient.player.pc.GetHardware())
                ),
                new byte[0],
                playerClient.player.pc.GetID(),
                "",
                new byte[0],
                false,
                "",
                -1,
                new byte[0],
                StringCompressor.Zip("{}"),
                false
            );
        }

        public static Message CreateContextRuntimeState(
            string id,
            List<ValMap> states,
            List<StackItem> stackTrace
        )
        {
            return new ContextRuntimeStateClientRpc(
                id,
                StringCompressor.Zip(OutputHelper.ToValMapsJson(states)),
                StringCompressor.Zip(JsonConvert.SerializeObject(stackTrace))
            );
        }

        public static Message CreateContextBreakpoint(
            string id,
            string filePath,
            int line,
            List<Greybel.RuntimeMetadata.DebugFrame> frames,
            List<DebugValueTree.Entry> globals,
            bool breakpoint
        )
        {
            return new ContextBreakpointRpc(
                id,
                filePath,
                line,
                StringCompressor.Zip(JsonConvert.SerializeObject(frames)),
                StringCompressor.Zip(JsonConvert.SerializeObject(globals)),
                breakpoint
            );
        }

        public static Message CreateContextOpenInternalFile(
            string id,
            string filePath,
            string source
        )
        {
            return new ContextOpenInternalFileRpc(id, filePath, StringCompressor.Zip(source));
        }

        public static Message CreateBuildState(
            string buildId,
            BuildManager.BuildState state,
            string errorMessage = "",
            string output = ""
        )
        {
            return new BuildStateClientRpc(
                buildId,
                (int)state,
                errorMessage,
                StringCompressor.Zip(output)
            );
        }

        public static Message CreateBuildCompletion(
            string buildId,
            BuildManager.BuildState state,
            string errorMessage = "",
            string output = ""
        )
        {
            return new BuildFinishedRpc(
                buildId,
                (int)state,
                errorMessage,
                StringCompressor.Zip(output)
            );
        }

        public static Message CreateDisposedBuild(
            string buildId,
            bool disposeSuccess,
            BuildManager.BuildState state,
            string errorMessage = "",
            string output = ""
        )
        {
            return new DisposedBuildRpc(
                buildId,
                disposeSuccess,
                (int)state,
                errorMessage,
                StringCompressor.Zip(output)
            );
        }

        public static Message CreateStatus()
        {
            return new StatusRpc(
                true,
                Proxy.GetPlayerClient() != null,
                Networking.IsSinglePlayer(),
                PluginInfo.PLUGIN_VERSION,
                GameVersion.Current
            );
        }
    }
}
