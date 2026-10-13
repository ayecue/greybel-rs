namespace GreyHackMessageHook.Tests;

public class RelevantGameMemberTableTests
{
    [Fact]
    public void ShouldFindEveryMemberThePluginReachesByReflection()
    {
        IReadOnlyList<string> absent = RelevantGameMemberTable.GetMissingMembers();

        Assert.Empty(absent);
    }
}
