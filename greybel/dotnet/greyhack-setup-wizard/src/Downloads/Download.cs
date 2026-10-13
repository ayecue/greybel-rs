using System.Security.Cryptography;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard.Downloads;

public static class Download
{
    public static async Task<string> FetchAsync(
        HttpClient http,
        Uri url,
        string fileName,
        string sha256,
        string cacheFolder
    )
    {
        Directory.CreateDirectory(cacheFolder);
        string path = Path.Combine(cacheFolder, fileName);
        if (File.Exists(path) && MatchesChecksum(path, sha256))
        {
            return path;
        }
        string partial = path + ".part";
        try
        {
            using (
                HttpResponseMessage response = await http.GetAsync(
                    url,
                    HttpCompletionOption.ResponseHeadersRead
                )
            )
            {
                if (!response.IsSuccessStatusCode)
                {
                    throw new SetupException(
                        Localization.Format(
                            Messages.DownloadStatusFailure,
                            url,
                            (int)response.StatusCode
                        )
                    );
                }
                await using FileStream file = File.Create(partial);
                await response.Content.CopyToAsync(file);
            }
            if (!MatchesChecksum(partial, sha256))
            {
                throw new SetupException(
                    Localization.Format(Messages.ChecksumMismatch, url, sha256, GetSha256(partial))
                );
            }
            File.Move(partial, path, overwrite: true);
            return path;
        }
        catch (HttpRequestException error)
        {
            throw new SetupException(
                Localization.Format(Messages.DownloadFailure, url, error.Message)
            );
        }
        finally
        {
            File.Delete(partial);
        }
    }

    public static bool MatchesChecksum(string path, string sha256) =>
        string.Equals(GetSha256(path), sha256, StringComparison.OrdinalIgnoreCase);

    public static string GetSha256(string path)
    {
        using FileStream file = File.OpenRead(path);
        return Convert.ToHexStringLower(SHA256.HashData(file));
    }
}
