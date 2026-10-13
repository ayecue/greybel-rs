using System;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Replaces kernels on routers in the player's network. <see cref="WorldRestore"/> saves each original kernel for
/// restoration during the next player setup.
/// </summary>
internal sealed class KernelFixture(RouterKernel kernels, WorldRestore restore)
{
    /// <summary>
    /// Selects a game-generated kernel version (<c>VersionsControl.GetAllLibs</c>) with the requested requirement,
    /// or generates one with <c>Library.Configure</c> if none matches. Returns the original content for restoration.
    /// </summary>
    public Reply SetRequirement(SetRouterKernelRequirementRpc request)
    {
        string original = kernels.Read(request.LocalIp);
        Library.Vulnerabilidad.Required requirement = (Library.Vulnerabilidad.Required)
            request.Requirement;

        foreach (string hash in VersionsControl.Singleton.GetAllLibs()["kernel_router.so"])
        {
            string content = Database.Singleton.GetContenido(hash);
            if (string.IsNullOrEmpty(content) || content == original)
            {
                continue;
            }
            Library library = JsonConvert.DeserializeObject<Library>(content);
            if (library != null && HasRequirement(library, requirement))
            {
                return Replace(request.LocalIp, original, content);
            }
        }

        // No existing world version matches, so generate a new version with the game's generator.
        for (int seed = 0; seed < 1000; seed++)
        {
            Library library = new Library();
            library.Configure(new Random(seed), VersionsControl.IdLib.KERNEL_ROUTER);
            // Assign the next version number after the newest, matching `Library.NewVersion`.
            GameField.LibraryVersion.SetValue(
                library,
                new VersionProgram(
                    VersionsControl.Singleton.GetLastLibs()["kernel_router.so"].GetVersion()
                )
            );
            library.NewVersion();
            if (HasRequirement(library, requirement))
            {
                return Replace(request.LocalIp, original, JsonConvert.SerializeObject(library));
            }
        }

        throw new InvalidOperationException("No kernel the generator makes has that requirement");
    }

    /// <summary>Writes test-supplied kernel content, including an original returned by <see cref="SetRequirement"/>.</summary>
    public Reply Write(WriteRouterKernelRpc request)
    {
        restore.RecordKernel(request.LocalIp);
        kernels.Write(request.LocalIp, request.Content);
        restore.RemoveRestoredKernel(request.LocalIp, request.Content);
        return new Reply.Applied();
    }

    private Reply Replace(string localIp, string original, string content)
    {
        restore.RecordKernel(localIp);
        kernels.Write(localIp, content);
        return new Reply.KernelReplaced(new KernelReplacedReply(original));
    }

    private static bool HasRequirement(
        Library library,
        Library.Vulnerabilidad.Required requirement
    ) =>
        library
            .GetMemoryZones()
            .Values.SelectMany(zone => zone.vulnerabs)
            .Any(weakness => weakness.requiredActions.Contains(requirement));
}
