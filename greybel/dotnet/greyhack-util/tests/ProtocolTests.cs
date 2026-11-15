using GreyHackUtil.Contracts;
using GreyHackUtil.Protocol;
using static GreyHackUtil.Tests.Support.ProtocolSetup;

namespace GreyHackUtil.Tests;

public class ProtocolTests
{
    [Fact]
    public void ShouldKeepTheOrderOfTheVersionBeforeAndAddNewMessagesAfter()
    {
        Direction direction = CreateDirection("NewRpc", "SecondRpc", "FirstRpc");

        var ordered = ProtocolWriter.Order(direction, ["FirstRpc", "SecondRpc"]);

        Assert.Equal(["FirstRpc", "SecondRpc", "NewRpc"], ordered.Select(message => message.Name));
    }

    [Fact]
    public void ShouldRefuseAMinorWithoutAMessageOfTheVersionBefore()
    {
        Direction direction = CreateDirection("FirstRpc");

        var ordering = () => ProtocolWriter.Order(direction, ["FirstRpc", "MissingRpc"]);

        var error = Assert.Throws<ContractException>(ordering);
        Assert.Contains("MissingRpc", error.Message);
    }

    [Theory]
    [InlineData("ssh", "Ssh")]
    [InlineData("bank_account", "BankAccount")]
    [InlineData("Administrative_Action", "AdministrativeAction")]
    [InlineData("EVERYONE", "Everyone")]
    [InlineData("LoginIssues", "LoginIssues")]
    public void ShouldNameMembersInPascalCase(string member, string expected)
    {
        string name = ProtocolNameTable.ToMemberName(member);

        Assert.Equal(expected, name);
    }

    [Theory]
    [InlineData("context_id", "Message", "ContextId")]
    [InlineData("write", "Message", "WriteField")]
    [InlineData("log", "Log", "LogField")]
    public void ShouldNameTheCSharpPropertiesAsDuplexCsDoes(
        string field,
        string owner,
        string expected
    )
    {
        string property = ProtocolNameTable.ToPropertyName(field, owner);

        Assert.Equal(expected, property);
    }
}
