using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Creates files on the player's computer.</summary>
internal sealed class ComputerFixture(Player player)
{
    /// <summary>Creates a text file owned by the first user of the player's computer.</summary>
    public Reply WriteFile(WriteFileRpc request)
    {
        string parent = request.Path.Substring(0, request.Path.LastIndexOf('/'));
        if (parent.Length == 0)
        {
            parent = "/";
        }
        string name = request.Path.Substring(request.Path.LastIndexOf('/') + 1);
        PlayerComputer computer = player.Computer;
        string owner = computer.GetUsers().FirstOrDefault()?.nombreUsuario ?? "root";
        computer
            .GetFileSystem()
            .ForceAddFile(
                new FileSystem.Archivo(name, request.Content, owner, false, true),
                parent
            );
        Database.Singleton.SyncFileSystemDB(computer, onlyFileSystem: true);
        return new Reply.FileWritten(new FileWrittenReply(request.Path));
    }
}
