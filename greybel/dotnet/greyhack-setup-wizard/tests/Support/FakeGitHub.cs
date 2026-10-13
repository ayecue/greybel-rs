using System.Net;
using System.Security.Cryptography;
using System.Text.Json;

namespace GreyHackSetupWizard.Tests.Support;

internal sealed class FakeGitHub : HttpMessageHandler
{
    public const string Repository = "test/greybel";
    public const string Tag = "v9.9.9";

    private readonly Dictionary<string, byte[]> files = [];

    public void AddFile(string name, byte[] content) => files[name] = content;

    protected override Task<HttpResponseMessage> SendAsync(
        HttpRequestMessage request,
        CancellationToken cancellationToken
    )
    {
        string url = request.RequestUri!.ToString();
        if (url == $"https://api.github.com/repos/{Repository}/releases/latest")
        {
            return Task.FromResult(
                files.Count == 0
                    ? new HttpResponseMessage(HttpStatusCode.NotFound)
                    : new HttpResponseMessage(HttpStatusCode.OK)
                    {
                        Content = new StringContent(CreateReleaseJson()),
                    }
            );
        }
        string prefix = GetDownloadPrefix();
        if (
            url.StartsWith(prefix, StringComparison.Ordinal)
            && files.TryGetValue(url[prefix.Length..], out byte[]? content)
        )
        {
            return Task.FromResult(
                new HttpResponseMessage(HttpStatusCode.OK)
                {
                    Content = new ByteArrayContent(content),
                }
            );
        }
        return Task.FromResult(new HttpResponseMessage(HttpStatusCode.NotFound));
    }

    private static string GetDownloadPrefix() =>
        $"https://github.com/{Repository}/releases/download/{Tag}/";

    private string CreateReleaseJson() =>
        JsonSerializer.Serialize(
            new
            {
                tag_name = Tag,
                assets = files.Select(file => new
                {
                    name = file.Key,
                    browser_download_url = GetDownloadPrefix() + file.Key,
                    digest = "sha256:" + Convert.ToHexStringLower(SHA256.HashData(file.Value)),
                }),
            }
        );
}
