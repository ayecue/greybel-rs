using GreyHackTestSupport.Game;
using GreyHackTestSupport.Platform;
using GreyHackTestSupport.Session;
using GreyHackTestSupport.Tests.Support;

namespace GreyHackTestSupport.Tests.Session;

public class SessionFileTests
{
    [Fact]
    public void ShouldReadNoLiveSessionWhoseHostIsMissing()
    {
        using var folder = new TemporaryFolder();
        var session = new SessionFile(new WorkFolder(folder.Path));
        session.Write(new SessionRecord(int.MaxValue, 4100, 4200, "player.log"));

        SessionRecord? live = session.ReadLive();

        Assert.NotNull(session.Read());
        Assert.Null(live);
    }

    [Fact]
    public void ShouldReadNoLiveSessionWhosePortsDontListen()
    {
        using var folder = new TemporaryFolder();
        var session = new SessionFile(new WorkFolder(folder.Path));
        int port = LocalPort.Resolve(0);
        session.Write(new SessionRecord(Environment.ProcessId, port, port, "player.log"));

        SessionRecord? live = session.ReadLive();

        Assert.Null(live);
    }

    [Fact]
    public void ShouldFindASessionIdleOnlyAfterItsTimeout()
    {
        using var folder = new TemporaryFolder();
        var session = new SessionFile(new WorkFolder(folder.Path));
        session.Write(new SessionRecord(Environment.ProcessId, 4100, 4200, "player.log"));
        session.UpdateLastUse();

        bool idleNow = session.IsIdle(TimeSpan.FromMinutes(1));
        bool idleWithoutTimeout = session.IsIdle(TimeSpan.FromMinutes(-1));

        Assert.False(idleNow);
        Assert.True(idleWithoutTimeout);
    }

    [Fact]
    public void ShouldClearTheStopRequestWithTheSession()
    {
        using var folder = new TemporaryFolder();
        var session = new SessionFile(new WorkFolder(folder.Path));
        session.Write(new SessionRecord(Environment.ProcessId, 4100, 4200, "player.log"));
        session.RequestStop();

        bool requested = session.IsStopRequested;
        session.Clear();

        Assert.True(requested);
        Assert.False(session.IsStopRequested);
        Assert.Null(session.Read());
    }
}
