using System.CommandLine;
using GreyHackTestSupport.Proxy;

namespace GreyHackTestSupport.Cli;

internal static class ProxyCommand
{
    public static Command Create()
    {
        Option<int> port = new("--port")
        {
            Description = "The proxy's port; 0 picks a free one",
            DefaultValueFactory = _ => 0,
        };

        var command = new Command(
            "proxy",
            "Run the message hook's proxy with standard streams as the game: prints {\"port\":N}, then each message a "
                + "client sends the game as the game's JSON line; each JSON line of standard input goes to every client"
        )
        {
            port,
        };
        command.SetAction(result => CommandAction.Run(() => ProxyHost.Run(result.GetValue(port))));
        return command;
    }
}
