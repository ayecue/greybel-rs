using System.Collections.Generic;
using System.Threading.Tasks;
using HarmonyLib;

namespace GreyHackMessageHook
{
    [HarmonyPatch(typeof(RamWidget))]
    public static class RamWidgetUpdateProcesosPatch
    {
        [HarmonyPostfix]
        [HarmonyPatch("UpdateProcesos")]
        static void Postfix_UpdateProcesos(
            RamWidget __instance,
            List<Computer.Proceso> origProcesos
        )
        {
            float ramUsedMb = 0f;
            for (int i = 0; i < origProcesos.Count; i++)
            {
                ramUsedMb += origProcesos[i].ramUsedMb;
            }
            RamManager.TotalRamUsage = ramUsedMb;
        }
    }

    public static class RamManager
    {
        public static float TotalRamUsage = 0f;

        public static bool HasEnoughRam(float requiredRam)
        {
            return (
                    PlayerClient.Singleton.player.pc.GetHardware().GetTotalMemoryRam()
                    - TotalRamUsage
                ) >= requiredRam;
        }
    }

    public static class BuildManager
    {
        public enum BuildState
        {
            Initial = 0,
            Allocation = 1,
            PreScript = 2,
            EntityCreation = 3,
            PostScript = 4,
            Cleanup = 5,
            Complete = 10,
            Failure = 20,
            Unknown = 30,
        }

        public enum ScriptType
        {
            Pre = 10,
            Post = 20,
        }

        public class BuildProcess
        {
            public class Resource
            {
                public readonly string location;
                public readonly string content;

                public Resource(string location, string content)
                {
                    this.location = location;
                    this.content = content;
                }
            }

            public class Script
            {
                public readonly ScriptType type;
                public readonly string content;

                public Script(ScriptType type, string content)
                {
                    this.type = type;
                    this.content = content;
                }
            }

            public readonly string BuildId;
            public readonly string Destination;
            public readonly string ResourceDestination;
            public readonly bool AutoCompile;

            private readonly HashSet<string> knownFolders = new HashSet<string> { "/" };
            public readonly List<Script> Scripts = new List<Script>();
            public readonly List<Resource> Resources = new List<Resource>();
            public readonly Dictionary<string, string> EnvironmentVariables =
                new Dictionary<string, string>();

            private BuildState state = BuildState.Initial;
            private string errorMessage = "";
            private string output = "";

            private WindowHandler codeEditorWindow = null;
            private WindowHandler terminalWindow = null;

            private PlayerClient client;

            public BuildProcess(
                string BuildId,
                string Destination,
                string ResourceDestination,
                bool AutoCompile,
                PlayerClient client
            )
            {
                this.BuildId = BuildId;
                this.Destination = Destination.TrimEnd('/');
                this.ResourceDestination = ResourceDestination.TrimEnd('/');
                this.AutoCompile = AutoCompile;
                this.client = client;

                InitializeEnvironment();
            }

            private void InitializeEnvironment()
            {
                EnvironmentVariables.Clear();
                EnvironmentVariables.Add(
                    "BUILD_RESOURCE_DESTINATION",
                    $"\"{ResourceDestination}\""
                );
                EnvironmentVariables.Add("BUILD_ID", $"\"{BuildId}\"");
                EnvironmentVariables.Add("BUILD_DESTINATION", $"\"{Destination}\"");
                EnvironmentVariables.Add("BUILD_AUTO_COMPILE", AutoCompile ? "1" : "0");
            }

            public BuildState GetState()
            {
                return state;
            }

            public string GetErrorMessage()
            {
                return errorMessage;
            }

            public string GetOutput()
            {
                return output;
            }

            public bool IsDone()
            {
                return (int)state >= (int)BuildState.Complete;
            }

            public string GetDestination()
            {
                if (AutoCompile)
                {
                    return ResourceDestination;
                }

                return Destination;
            }

            public void AddResource(string location, string content)
            {
                Resources.Add(new Resource(location, content));
            }

            public void AddScript(ScriptType type, string script)
            {
                Scripts.Add(new Script(type, script));
            }

            private string InjectEnvironmentVariables(string script)
            {
                string fileHeader = "";
                foreach (var pair in EnvironmentVariables)
                {
                    fileHeader += $"{pair.Key}={pair.Value};";
                }
                return fileHeader + script;
            }

            private async Task Allocate()
            {
                try
                {
                    float potentialUsedRam =
                        Computer.GetRamUsage("CodeEditor") + Computer.GetRamUsage("Terminal");

                    Logger.Log(
                        $"[Build {BuildId}] Current RAM used: {RamManager.TotalRamUsage} MB"
                    );

                    if (!RamManager.HasEnoughRam(potentialUsedRam))
                    {
                        throw new System.Exception("Not enough RAM to start build process");
                    }

                    Logger.Log($"[Build {BuildId}] Opening CodeEditor.exe");
                    codeEditorWindow = await WindowHandler.OpenWindow(client, "CodeEditor.exe");

                    Logger.Log($"[Build {BuildId}] Looking for bash in /bin");
                    FileSystem.Carpeta folder = await codeEditorWindow.GetFolder("/bin");
                    FileSystem.Archivo bashFile = folder.files.Find(f => f.GetNombre() == "bash");
                    string tempBashFileName = BuildHelper.GenerateRandomString(
                        BuildHelper.TemporaryFileNameLength
                    );

                    if (bashFile != null)
                    {
                        Logger.Log(
                            $"[Build {BuildId}] Found bash, moving to /bin/{tempBashFileName}"
                        );
                        await codeEditorWindow.MoveFile("/bin/bash", "/bin", tempBashFileName);
                    }

                    Logger.Log($"[Build {BuildId}] Opening Terminal.exe");
                    terminalWindow = await WindowHandler.OpenWindow(client, "Terminal.exe");

                    if (bashFile != null)
                    {
                        Logger.Log($"[Build {BuildId}] Moving bash back to /bin/bash");
                        await codeEditorWindow.MoveFile($"/bin/{tempBashFileName}", "/bin", "bash");
                    }

                    Logger.Log($"[Build {BuildId}] Allocation complete");

                    state = BuildState.PreScript;
                }
                catch (System.Exception e)
                {
                    state = BuildState.Failure;
                    errorMessage = e.Message;
                    Logger.LogError("Build process failed: " + e.ToString());
                }
            }

            private async Task RunPreScripts()
            {
                try
                {
                    EnvironmentVariables.Add("BUILD_STAGE", $"\"pre\"");

                    foreach (var script in Scripts)
                    {
                        if (script.type == ScriptType.Pre)
                        {
                            ScriptExecutionResult result =
                                await terminalWindow.RunTemporaryScriptInTerminal(
                                    InjectEnvironmentVariables(script.content)
                                );

                            output += result.Output + "\n";

                            if (result.ExitCode != 0)
                            {
                                throw new System.Exception(
                                    $"Pre-script failed with exit code {result.ExitCode}: {result.ExitMessage}"
                                );
                            }
                        }
                    }
                    state = BuildState.EntityCreation;
                }
                catch (System.Exception e)
                {
                    state = BuildState.Failure;
                    errorMessage = e.Message;
                    Logger.LogError("Build process failed: " + e.ToString());
                }
                finally
                {
                    EnvironmentVariables.Remove("BUILD_STAGE");
                }
            }

            private async Task CreateResources()
            {
                try
                {
                    foreach (var entity in Resources)
                    {
                        string path = GetDestination() + entity.location;
                        await EnsureFolder(PosixPath.GetDirectoryName(path));
                        await codeEditorWindow.AddResource(entity.content, path);
                    }
                    state = BuildState.PostScript;
                }
                catch (System.Exception e)
                {
                    state = BuildState.Failure;
                    errorMessage = e.Message;
                    Logger.LogError("Build process failed: " + e.ToString());
                }
            }

            private async Task RunPostScripts()
            {
                try
                {
                    EnvironmentVariables.Add("BUILD_STAGE", $"\"post\"");

                    foreach (var script in Scripts)
                    {
                        if (script.type == ScriptType.Post)
                        {
                            ScriptExecutionResult result =
                                await terminalWindow.RunTemporaryScriptInTerminal(
                                    InjectEnvironmentVariables(script.content)
                                );

                            output += result.Output + "\n";

                            if (result.ExitCode != 0)
                            {
                                throw new System.Exception(
                                    $"Post-script failed with exit code {result.ExitCode}: {result.ExitMessage}"
                                );
                            }
                        }
                    }
                    state = BuildState.Cleanup;
                }
                catch (System.Exception e)
                {
                    state = BuildState.Failure;
                    errorMessage = e.Message;
                    Logger.LogError("Build process failed: " + e.ToString());
                }
                finally
                {
                    EnvironmentVariables.Remove("BUILD_STAGE");
                }
            }

            private async Task EnsureFolder(string path)
            {
                if (knownFolders.Contains(path))
                {
                    return;
                }

                string parent = PosixPath.GetDirectoryName(path);
                await EnsureFolder(parent);

                string name = PosixPath.GetFileName(path);
                FileSystem.Carpeta contents = await codeEditorWindow.GetFolder(parent);
                if (!contents.folders.Exists(folder => folder.GetNombre() == name))
                {
                    Logger.Log($"[Build {BuildId}] Creating folder {path}");
                    await codeEditorWindow.CreateFolder(parent, name);
                }
                knownFolders.Add(path);
            }

            private async Task CloseWindows()
            {
                try
                {
                    if (codeEditorWindow != null)
                    {
                        await codeEditorWindow.Close();
                        codeEditorWindow = null;
                    }

                    if (terminalWindow != null)
                    {
                        await terminalWindow.Close();
                        terminalWindow = null;
                    }

                    state = BuildState.Complete;
                    Logger.Log($"[Build {BuildId}] Build process complete");
                }
                catch (System.Exception e)
                {
                    state = BuildState.Failure;
                    errorMessage = e.Message;
                    Logger.LogError("Build process failed: " + e.ToString());
                }
            }

            private IEnumerable<Task> Process()
            {
                Logger.Log($"[Build {BuildId}] Processing state {state}");

                switch (state)
                {
                    case BuildState.Allocation:
                    {
                        yield return Allocate();
                        break;
                    }
                    case BuildState.PreScript:
                    {
                        yield return RunPreScripts();
                        break;
                    }
                    case BuildState.EntityCreation:
                    {
                        yield return CreateResources();
                        break;
                    }
                    case BuildState.PostScript:
                    {
                        yield return RunPostScripts();
                        break;
                    }
                    case BuildState.Cleanup:
                    {
                        yield return CloseWindows();
                        break;
                    }
                    case BuildState.Complete:
                    case BuildState.Failure:
                    case BuildState.Unknown:
                    default:
                    {
                        yield return null;
                        break;
                    }
                }
            }

            public async Task Run()
            {
                if (state != BuildState.Initial)
                {
                    Logger.LogError("Build process already started or finished");
                    return;
                }

                state = BuildState.Allocation;
                Logger.Log($"[Build {BuildId}] Starting build process");

                while (!IsDone())
                {
                    foreach (var task in Process())
                    {
                        await task;
                    }
                }
            }

            public async Task Dispose()
            {
                if (codeEditorWindow != null)
                {
                    await codeEditorWindow.Dispose();
                    codeEditorWindow = null;
                }

                if (terminalWindow != null)
                {
                    await terminalWindow.Dispose();
                    terminalWindow = null;
                }
            }
        }

        private static Dictionary<string, BuildProcess> buildRegistry =
            new Dictionary<string, BuildProcess>();

        public static string CreateBuild(
            string id,
            string Destination,
            string ResourceDestination,
            bool AutoCompile,
            PlayerClient client
        )
        {
            BuildProcess process = new BuildProcess(
                id,
                Destination,
                ResourceDestination,
                AutoCompile,
                client
            );

            buildRegistry.Add(id, process);

            return id;
        }

        public static BuildProcess GetBuild(string id)
        {
            BuildProcess process = null;
            buildRegistry.TryGetValue(id, out process);
            return process;
        }

        public static bool DisposeBuild(string id)
        {
            BuildProcess process = null;
            buildRegistry.TryGetValue(id, out process);

            if (process == null)
            {
                return false;
            }

            process.Dispose();

            return buildRegistry.Remove(id);
        }
    }
}
