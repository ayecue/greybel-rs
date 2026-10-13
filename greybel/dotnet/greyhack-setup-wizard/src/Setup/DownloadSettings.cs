namespace GreyHackSetupWizard.Setup;

public sealed record DownloadSettings(HttpClient Http, string CacheFolder)
{
    public static string GetDefaultCacheFolder() =>
        Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "greyhack-setup-wizard"
        );
}
