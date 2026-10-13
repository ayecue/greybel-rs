using System.CommandLine;
using GreyHackTestSupport.Cli;

var root = new RootCommand("Run Grey Hack and the message hook's proxy for greybel's tests")
{
    GameCommand.Create(),
    ProxyCommand.Create(),
};
return root.Parse(args).Invoke();
