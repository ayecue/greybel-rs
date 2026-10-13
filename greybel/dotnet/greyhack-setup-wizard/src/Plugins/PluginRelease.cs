using System.Net.Http.Headers;
using System.Text.Json;
using GreyHackSetupWizard.BepInEx;
using GreyHackSetupWizard.Downloads;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Plugins;

public sealed class PluginRelease
{
    public const string DefaultRepository = "ayecue/greybel-rs";

    private readonly IReadOnlyDictionary<string, ReleaseAsset> assets;

    private PluginRelease(string tag, IReadOnlyDictionary<string, ReleaseAsset> assets)
    {
        Tag = tag;
        this.assets = assets;
    }

    public string Tag { get; }

    public static async Task<PluginRelease> FindAsync(
        HttpClient http,
        string repository,
        string? tag
    )
    {
        string path = tag == null ? "latest" : "tags/" + Uri.EscapeDataString(tag);
        using var request = new HttpRequestMessage(
            HttpMethod.Get,
            $"https://api.github.com/repos/{repository}/releases/{path}"
        );
        request.Headers.UserAgent.Add(new ProductInfoHeaderValue("greyhack-setup-wizard", "1.0"));
        request.Headers.Accept.Add(
            new MediaTypeWithQualityHeaderValue("application/vnd.github+json")
        );
        try
        {
            using HttpResponseMessage response = await http.SendAsync(request);
            if (!response.IsSuccessStatusCode)
            {
                throw new SetupException(
                    tag == null
                        ? Localization.Format(
                            Messages.ReleaseMissing,
                            repository,
                            (int)response.StatusCode
                        )
                        : Localization.Format(
                            Messages.ReleaseTagMissing,
                            tag,
                            repository,
                            (int)response.StatusCode
                        )
                );
            }
            return Parse(await response.Content.ReadAsStringAsync());
        }
        catch (HttpRequestException error)
        {
            throw new SetupException(
                Localization.Format(Messages.ReleasesReadFailure, repository, error.Message)
            );
        }
    }

    public static PluginRelease Parse(string json)
    {
        using JsonDocument document = JsonDocument.Parse(json);
        JsonElement release = document.RootElement;
        var assets = new Dictionary<string, ReleaseAsset>();
        foreach (JsonElement asset in release.GetProperty("assets").EnumerateArray())
        {
            string name = asset.GetProperty("name").GetString()!;
            string? digest = asset.TryGetProperty("digest", out JsonElement value)
                ? value.GetString()
                : null;
            assets[name] = new ReleaseAsset(
                name,
                new Uri(asset.GetProperty("browser_download_url").GetString()!),
                digest != null && digest.StartsWith("sha256:", StringComparison.Ordinal)
                    ? digest["sha256:".Length..]
                    : null
            );
        }
        return new PluginRelease(release.GetProperty("tag_name").GetString()!, assets);
    }

    public ReleaseAsset GetAsset(PluginBuild plugin, BepInExMajor major)
    {
        string name = plugin.GetAssemblyName(major) + ".dll";
        if (!assets.TryGetValue(name, out ReleaseAsset? asset))
        {
            throw new SetupException(Localization.Format(Messages.ReleaseAssetMissing, Tag, name));
        }
        if (asset.Sha256 == null)
        {
            throw new SetupException(
                Localization.Format(Messages.ReleaseChecksumMissing, Tag, name)
            );
        }
        return asset;
    }

    public Task<string> FetchAsync(
        HttpClient http,
        PluginBuild plugin,
        BepInExMajor major,
        string cacheFolder
    )
    {
        ReleaseAsset asset = GetAsset(plugin, major);
        return Download.FetchAsync(
            http,
            asset.Url,
            asset.Name,
            asset.Sha256!,
            Path.Combine(cacheFolder, Tag)
        );
    }
}

public sealed record ReleaseAsset(string Name, Uri Url, string? Sha256);
