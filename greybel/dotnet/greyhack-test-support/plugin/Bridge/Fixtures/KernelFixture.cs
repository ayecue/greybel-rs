using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Replaces kernels on routers in the player's network. <see cref="WorldRestore"/> saves each original kernel for
/// restoration during the next player setup.
/// </summary>
internal sealed class KernelFixture(
    RouterKernel kernels,
    WorldRestore restore,
    NetworkFixture networks
)
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

        restore.RecordKernel(request.LocalIp);
        networks.InstallFixture(
            new InstallFixtureLibraryRpc(
                0,
                request.LocalIp,
                "kernel_router.so",
                new[]
                {
                    new FixtureWeakness(
                        (byte)Library.Vulnerabilidad.HackResult.RANDOM_FOLDER,
                        new[] { request.Requirement },
                        "",
                        ""
                    ),
                },
                false
            )
        );
        return new Reply.KernelReplaced(new KernelReplacedReply(original));
    }

    /// <summary>Writes test-supplied kernel content, including an original returned by <see cref="SetRequirement"/>.</summary>
    public Reply Write(WriteRouterKernelRpc request)
    {
        restore.RecordKernel(request.LocalIp);
        kernels.Write(request.LocalIp, request.Content);
        restore.RemoveRestoredKernel(request.LocalIp, request.Content);
        return new Reply.Applied();
    }
}
