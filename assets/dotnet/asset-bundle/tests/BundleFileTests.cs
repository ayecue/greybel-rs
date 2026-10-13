using AssetBundle.Tests.Support;

namespace AssetBundle.Tests;

public class BundleFileTests
{
    [Fact]
    public void ShouldReadBackEveryFileWritten()
    {
        byte[] bundle = BundleFile.Write(Fixtures.Files);

        var files = BundleFile.Read(bundle);

        Assert.Equal(Fixtures.AsHex(Fixtures.Files), Fixtures.AsHex(files));
    }

    [Fact]
    public void ShouldReadABundleTheRustWriterWrote()
    {
        byte[] bundle = Fixtures.Read("rust.bundle");

        var files = BundleFile.Read(bundle);

        Assert.Equal(Fixtures.AsHex(Fixtures.Files), Fixtures.AsHex(files));
    }

    [Fact]
    public void ShouldWriteAnEmptyFileAsACompleteDeflateStream()
    {
        (string, byte[])[] files = [("empty", [])];

        byte[] bundle = BundleFile.Write(files);

        Assert.Equal([0x03, 0x00], bundle[^2..]);
    }

    [Fact]
    public void ShouldRefuseBytesWithoutTheMagic()
    {
        byte[] bytes = "PNG\0\0\0\0\0\0"u8.ToArray();

        var error = Record.Exception(() => BundleFile.Read(bytes));

        Assert.IsType<InvalidDataException>(error);
    }
}
