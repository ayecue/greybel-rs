using GreyHackTestSupport.Game;
using GreyHackTestSupport.Session;

namespace GreyHackTestSupport.Tests.Session;

public class SessionRecordTests
{
    [Fact]
    public void ShouldWriteASessionRecordWithItsHostFirst()
    {
        var record = SessionRecord.Create(7, new GameConnection(4100, 4200, "/work/player.log"));

        string json = record.ToJson();

        Assert.Equal(
            "{\"pid\":7,\"port\":4100,\"testPort\":4200,\"log\":\"/work/player.log\"}",
            json
        );
    }

    [Fact]
    public void ShouldReadBackASessionRecord()
    {
        var record = new SessionRecord(7, 4100, 4200, "/work/player.log");

        SessionRecord? parsed = SessionRecord.Parse(record.ToJson());

        Assert.Equal(record, parsed);
    }

    [Fact]
    public void ShouldReadNoSessionRecordFromATornFile()
    {
        SessionRecord? parsed = SessionRecord.Parse("{\"pid\":7,\"po");

        Assert.Null(parsed);
    }
}
