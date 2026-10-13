using GreyHackUtil.Contracts;
using GreyHackUtil.Protocol;
using static GreyHackUtil.Tests.Support.ContractFixtures;

namespace GreyHackUtil.Tests;

public class ContractSetTests
{
    [Fact]
    public void ShouldMergeGameAndExtensionMessages()
    {
        ContractFile game = CreateGame(
            CreateMessage(
                "CatServerRpc",
                124,
                ("pathFile", "System.String"),
                ("terminalPID", "System.Int32")
            ),
            new MessageContract { Name = "IgnoredServerRpc", Id = 3 }
        );
        var extension = new ContractFile { Server = [CreateMessage("HealthCheckRpc", 1200)] };

        var contracts = new ContractSet(game, extension);

        Assert.Equal(
            ["CatServerRpc", "HealthCheckRpc"],
            contracts.Server.TypedMessages.Select(m => m.Name)
        );
        Assert.False(contracts.Server.Messages.Single(m => m.Name == "IgnoredServerRpc").IsTyped);
    }

    [Fact]
    public void ShouldRejectIdsUsedTwice()
    {
        ContractFile game = CreateGame(CreateMessage("CatServerRpc", 1000));
        var extension = new ContractFile { Server = [CreateMessage("CreateContextRpc", 1000)] };

        Exception? exception = Record.Exception(() => new ContractSet(game, extension));

        var error = Assert.IsType<ContractException>(exception);
        Assert.Contains("id 1000", error.Message);
    }

    [Fact]
    public void ShouldRejectUnknownFieldTypes()
    {
        ContractFile game = CreateGame(
            CreateMessage("CatServerRpc", 124, ("pathFile", "nopeValues"))
        );
        var extension = new ContractFile();

        Exception? exception = Record.Exception(() => new ContractSet(game, extension));

        var error = Assert.IsType<ContractException>(exception);
        Assert.Contains("nopeValues", error.Message);
    }

    [Fact]
    public void ShouldTagEveryTypedMessageWithItsGameIdAndItsFieldsWithTheirLists()
    {
        ContractSet contracts = CreateTypedCat();

        string protocol = ProtocolWriter.Generate(contracts);

        Assert.Contains("@game_message_id(124)\n        message CatServerRpc {", protocol);
        Assert.Contains("@game_param_type(\"System.Byte[]\") zip_content: bytes,", protocol);
        Assert.Contains("@game_param_type(\"System.Int32\")", protocol);
    }

    [Fact]
    public void ShouldRejectFieldsTheProtocolCannotDeclare()
    {
        var twice = new ContractSet(
            CreateGame(
                CreateMessage(
                    "CatServerRpc",
                    124,
                    ("windowPID", "System.Int32"),
                    ("windowPid", "System.Int32")
                )
            ),
            new ContractFile()
        );

        Exception? exception = Record.Exception(() => ProtocolWriter.Generate(twice));

        Assert.IsType<ContractException>(exception);
    }

    [Fact]
    public void ShouldReadTheWindowFieldOfAMessage()
    {
        ContractFile game = CreateClient(CreateWindowedPrint());

        var contracts = new ContractSet(game, new ContractFile());

        Assert.Equal("windowPID", contracts.Client.Messages.Single().Window?.Name);
    }

    [Fact]
    public void ShouldTagTheWindowFieldOfAMessage()
    {
        ContractSet contracts = CreateWindowedPrintSet();

        string protocol = ProtocolWriter.Generate(contracts);

        Assert.Contains("@game_param_type(\"System.Int32\") @pid window_pid: i32,", protocol);
    }

    [Fact]
    public void ShouldRejectWindowFieldsThatHoldNoPids()
    {
        MessageContract print = CreateMessage(
            "PrintSentClientRpc",
            80,
            ("message", "System.String")
        );
        print.Window = "message";
        ContractFile game = CreateClient(print);

        Exception? exception = Record.Exception(() => new ContractSet(game, new ContractFile()));

        var error = Assert.IsType<ContractException>(exception);
        Assert.Contains("window field message", error.Message);
    }
}
