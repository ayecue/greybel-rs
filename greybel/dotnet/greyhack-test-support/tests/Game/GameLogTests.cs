using GreyHackTestSupport.Game;
using GreyHackTestSupport.Tests.Support;

namespace GreyHackTestSupport.Tests.Game;

public class GameLogTests
{
    [Fact]
    public void ShouldFindTheReadyLineWhileTheGameWritesTheLog()
    {
        using var folder = new TemporaryFolder();
        string log = folder.GetPath("player.log");
        using var writer = new StreamWriter(
            new FileStream(log, FileMode.Create, FileAccess.Write, FileShare.ReadWrite)
        )
        {
            AutoFlush = true,
        };
        writer.WriteLine("Player login OK");

        bool before = GameLog.Contains(log, GameLog.DesktopReady);
        writer.WriteLine(GameLog.DesktopReady);
        bool after = GameLog.Contains(log, GameLog.DesktopReady);

        Assert.False(before);
        Assert.True(after);
    }

    [Fact]
    public void ShouldTailTheLogWithoutTheShaderComplaints()
    {
        using var folder = new TemporaryFolder();
        string log = folder.GetPath("player.log");
        File.WriteAllText(
            log,
            "one\nShader Hidden/Blit is not supported\ntwo\r\n#pragma warning\nthree\n"
        );

        IReadOnlyList<string> tail = GameLog.ReadTail(log, 3);

        Assert.Equal(["two", "three", ""], tail);
    }

    [Fact]
    public void ShouldReadALogNotWrittenYetAsEmpty()
    {
        using var folder = new TemporaryFolder();

        bool contains = GameLog.Contains(folder.GetPath("player.log"), GameLog.DesktopReady);

        Assert.False(contains);
    }
}
