using System.CommandLine;
using GreyHackUtil.Cli;

var root = new RootCommand(
    "Extract Grey Hack's network contracts and assets, and generate the duplex protocol of its messages from the contracts"
)
{
    ExtractCommand.Create(),
    AssetsCommand.Create(),
    GenerateCommand.Create(),
};
return root.Parse(args).Invoke();
