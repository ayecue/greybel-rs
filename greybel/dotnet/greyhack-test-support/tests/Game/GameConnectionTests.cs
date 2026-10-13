using GreyHackTestSupport.Game;

namespace GreyHackTestSupport.Tests.Game;

public class GameConnectionTests
{
    [Fact]
    public void ShouldWriteTheJsonTheTestsRead()
    {
        var connection = new GameConnection(4100, 4200, "/work/player.log");

        string json = connection.ToJson();

        Assert.Equal("{\"port\":4100,\"testPort\":4200,\"log\":\"/work/player.log\"}", json);
    }
}
