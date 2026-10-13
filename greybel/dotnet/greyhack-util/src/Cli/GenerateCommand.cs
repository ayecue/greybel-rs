using System.CommandLine;
using GreyHackUtil.Contracts;
using GreyHackUtil.Protocol;

namespace GreyHackUtil.Cli;

internal static class GenerateCommand
{
    public static Command Create()
    {
        Option<DirectoryInfo> contracts = new Option<DirectoryInfo>("--contracts", "-c")
        {
            Description =
                $"Directory with {ContractSet.GameBundle} and {ContractSet.ExtensionFile}",
            Required = true,
        }.AcceptExistingOnly();
        Option<FileInfo> output = new("--output", "-o")
        {
            Description = "The protocol file to write, protocols/greyhack/<major>.<minor>.protocol",
            Required = true,
        };

        var command = new Command(
            "generate",
            "Write the duplex protocol of the game's messages from the contracts"
        )
        {
            contracts,
            output,
        };
        command.SetAction(result =>
            CommandAction.Run(() =>
            {
                ContractSet set = ContractSet.Load(result.GetRequiredValue(contracts).FullName);
                string path = result.GetRequiredValue(output).FullName;
                File.WriteAllText(path, ProtocolWriter.Generate(set, path));
            })
        );
        return command;
    }
}
