using AssetBundle;
using GreyHackUtil.Contracts;
using GreyHackUtil.Extraction;
using GreyHackUtil.Protocol;
using GreyHackUtil.Tests.Support;
using static GreyHackUtil.Tests.Support.Repository;

namespace GreyHackUtil.Tests;

public class RepositoryTests
{
    [Fact]
    public void ShouldKeepTheGeneratedProtocolUpToDate()
    {
        ContractSet contracts = ContractSet.Load(ContractDirectory);
        string written = File.ReadAllText(ProtocolFile);

        string generated = ProtocolWriter.Generate(contracts, ProtocolFile);

        Assert.True(
            generated == written,
            $"{ProtocolFile} is out of date: run `make generate` in greybel/dotnet/"
        );
    }

    [Fact]
    public void ShouldRoundTripTheGameContract()
    {
        string written = GetGameContract();

        string serialized = ContractJson.Serialize(ContractSet.LoadGame(ContractDirectory));

        Assert.Equal(written, serialized);
    }

    [Fact]
    public void ShouldMatchTheGameContractToItsFingerprint()
    {
        string written = File.ReadAllText(Path.Combine(ContractDirectory, Fingerprint.FileName));
        ContractFile game = ContractSet.LoadGame(ContractDirectory);

        string fingerprint = Fingerprint.Compute(
            game.GameVersion!,
            [(ContractSet.GameFile, System.Text.Encoding.UTF8.GetBytes(GetGameContract()))]
        );

        Assert.Equal(written, fingerprint);
    }

    [Fact]
    public void ShouldMatchTheGameContractToTheInstalledGame()
    {
        string? assembly = FindInstalledAssembly();
        Assert.SkipWhen(
            assembly == null,
            "the game is not installed (set GREY_HACK_PATH or GREY_HACK_MANAGED_PATH)"
        );
        string written = GetGameContract();

        string extracted = ContractJson.Serialize(ContractExtractor.Extract(assembly!, []));

        Assert.True(
            extracted == written,
            $"{ContractSet.GameBundle} differs from the installed game: run `make contracts` in greybel/dotnet/"
        );
    }
}
