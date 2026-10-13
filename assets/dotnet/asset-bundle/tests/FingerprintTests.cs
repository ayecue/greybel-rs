using AssetBundle.Tests.Support;

namespace AssetBundle.Tests;

public class FingerprintTests
{
    [Fact]
    public void ShouldFingerprintLikeTheRustCrate()
    {
        string expected = File.ReadAllText(Path.Combine(Fixtures.Folder, Fingerprint.FileName));

        string fingerprint = Fingerprint.Compute("1.0", Fixtures.Files);

        Assert.Equal(expected, fingerprint);
    }
}
