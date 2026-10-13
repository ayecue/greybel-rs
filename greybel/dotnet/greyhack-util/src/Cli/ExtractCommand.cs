using System.CommandLine;
using GreyHackUtil.Contracts;
using GreyHackUtil.Extraction;

namespace GreyHackUtil.Cli;

internal static class ExtractCommand
{
    public static Command Create()
    {
        Argument<FileInfo> assembly = new Argument<FileInfo>("assembly")
        {
            Description = "The game's Assembly-CSharp.dll",
        }.AcceptExistingOnly();
        Option<DirectoryInfo> output = new("--output", "-o")
        {
            Description =
                $"Folder to write the contract into ({ContractSet.GameBundle} and its fingerprint.json)",
            Required = true,
        };

        var command = new Command("extract", "Read the game's network contracts from its assembly")
        {
            assembly,
            output,
        };
        command.SetAction(result =>
            CommandAction.Run(() =>
            {
                var warnings = new List<string>();
                ContractFile contract = ContractExtractor.Extract(
                    result.GetRequiredValue(assembly).FullName,
                    warnings
                );
                foreach (string warning in warnings)
                {
                    Console.Error.WriteLine($"warning: {warning}");
                }
                ContractSet.SaveGame(contract, result.GetRequiredValue(output).FullName);
            })
        );
        return command;
    }
}
