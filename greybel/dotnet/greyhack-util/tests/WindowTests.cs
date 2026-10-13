using GreyHackUtil.Contracts;
using GreyHackUtil.Extraction;
using GreyHackUtil.Tests.Support;
using static GreyHackUtil.Tests.Support.WindowSetup;

namespace GreyHackUtil.Tests;

public class WindowTests
{
    [Theory]
    [InlineData("Finder", "SendsItsOwnPid")]
    [InlineData("Finder", "SendsItsOwnPidField")]
    [InlineData("Finder", "SendsItsOwnPidThroughALocal")]
    [InlineData("Finder", "SendsAnotherWindowPid")]
    [InlineData("Finder", "SendsAPassedPid")]
    [InlineData("Panel", "SendsAPidOfAWindowFromAField")]
    public void ShouldFollowAPidOfAWindowToGetPid(string type, string method)
    {
        var (reader, value, scope) = WindowGame.FindSent(type, method);

        bool pid = reader.HasPid(value, scope);

        Assert.True(pid);
    }

    [Theory]
    [InlineData("Finder", "SendsANumber")]
    [InlineData("Finder", "SendsAPassedNumber")]
    [InlineData("Finder", "SendsADelegatesPid")]
    [InlineData("Panel", "SendsANumberFromAField")]
    public void ShouldTakeNoOtherValueForAWindowPid(string type, string method)
    {
        var (reader, value, scope) = WindowGame.FindSent(type, method);

        bool pid = reader.HasPid(value, scope);

        Assert.False(pid);
    }

    [Theory]
    [InlineData("SendsItsOwnPid", true)]
    [InlineData("SendsItsOwnPidField", true)]
    [InlineData("SendsItsOwnPidThroughALocal", true)]
    [InlineData("SendsAnotherWindowPid", false)]
    [InlineData("SendsANumber", false)]
    public void ShouldDistinguishAWindowOwnPidFromAnotherWindowPid(string method, bool expected)
    {
        var (reader, value, scope) = WindowGame.FindSent("Finder", method);

        bool own = reader.HasOwnPid(value, scope);

        Assert.Equal(expected, own);
    }

    [Fact]
    public void ShouldTakeTheSendersOwnPidOverItsParentsForTheWindow()
    {
        var message = new MessageContract { Name = "UpdateSettingsServerRpc" };
        FieldContract[] fields =
        [
            CreateIntField("port"),
            CreateIntField("parentPID"),
            CreateIntField("windowPID"),
        ];
        SentLayout layout = CreateSent(
            CreateValue(),
            CreateValue(window: true),
            CreateValue(window: true, own: true)
        );
        var warnings = new List<string>();

        string? window = WindowReader.FromSenders(message, fields, [layout], warnings);

        Assert.Equal("windowPID", window);
        Assert.Empty(warnings);
    }

    [Fact]
    public void ShouldTakeTheOneWindowsPidWhenNoneIsTheSendersOwn()
    {
        var message = new MessageContract { Name = "JoinGuildServerRpc" };
        FieldContract[] fields = [CreateIntField("guildId"), CreateIntField("chatPID")];
        SentLayout layout = CreateSent(CreateValue(), CreateValue(window: true));

        string? window = WindowReader.FromSenders(message, fields, [layout], []);

        Assert.Equal("chatPID", window);
    }

    [Fact]
    public void ShouldGiveNoWindowWhenTheSendersDisagree()
    {
        var message = new MessageContract { Name = "RemoveSellHardwareServerRpc" };
        FieldContract[] fields = [CreateIntField("storageWindowPID")];

        string? window = WindowReader.FromSenders(
            message,
            fields,
            [CreateSent(CreateValue(window: true)), CreateSent(CreateValue())],
            []
        );

        Assert.Null(window);
    }

    [Fact]
    public void ShouldGiveNoWindowAndWarnForTwoOwnPids()
    {
        var message = new MessageContract { Name = "TwoWindowsServerRpc" };
        FieldContract[] fields = [CreateIntField("windowPID"), CreateIntField("otherPID")];
        SentLayout layout = CreateSent(
            CreateValue(window: true, own: true),
            CreateValue(window: true, own: true)
        );
        var warnings = new List<string>();

        string? window = WindowReader.FromSenders(message, fields, [layout], warnings);

        Assert.Null(window);
        Assert.Single(warnings);
    }
}
