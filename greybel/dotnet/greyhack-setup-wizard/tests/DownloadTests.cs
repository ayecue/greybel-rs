using GreyHackSetupWizard.Downloads;
using GreyHackSetupWizard.Tests.Support;

namespace GreyHackSetupWizard.Tests;

public class DownloadTests
{
    [Fact]
    public async Task ShouldReuseACachedFileOfTheExpectedSha256()
    {
        using var cache = new TemporaryFolder();
        File.WriteAllText(cache.GetPath("BepInEx.zip"), "archive");
        string sha256 = Download.GetSha256(cache.GetPath("BepInEx.zip"));
        using var http = new HttpClient();

        string path = await Download.FetchAsync(
            http,
            new Uri("http://127.0.0.1:9/never"),
            "BepInEx.zip",
            sha256,
            cache.Path
        );

        Assert.Equal(cache.GetPath("BepInEx.zip"), path);
    }

    [Fact]
    public async Task ShouldReportAFailedDownload()
    {
        using var cache = new TemporaryFolder();
        using var http = new HttpClient();

        var error = await Assert.ThrowsAsync<SetupException>(() =>
            Download.FetchAsync(
                http,
                new Uri("http://127.0.0.1:9/never"),
                "BepInEx.zip",
                new string('0', 64),
                cache.Path
            )
        );

        Assert.StartsWith("Couldn't download http://127.0.0.1:9/never", error.Message);
        Assert.False(File.Exists(cache.GetPath("BepInEx.zip.part")));
    }
}
