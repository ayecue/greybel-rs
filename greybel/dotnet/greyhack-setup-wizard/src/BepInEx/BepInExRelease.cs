namespace GreyHackSetupWizard.BepInEx;

public sealed record BepInExRelease(
    BepInExMajor Major,
    string Version,
    Platform Platform,
    string FileName,
    Uri Url,
    string Sha256
)
{
    private const string StableVersion = "5.4.23.6";
    private const string StableDownloadUrl =
        "https://github.com/BepInEx/BepInEx/releases/download/v5.4.23.6/";

    private const string BetaVersion = "6.0.0-be.791";
    private const string BetaBuildVersion = "6.0.0-be.791+b782da4";
    private const string BetaDownloadUrl = "https://builds.bepinex.dev/projects/bepinex_be/791/";

    public static readonly IReadOnlyList<BepInExRelease> All =
    [
        CreateStable(
            Platform.Windows,
            "BepInEx_win_x64_5.4.23.6.zip",
            "6c57a714a892efb4e9e1fef63d608a3391f30ec984031c465404e4ee381a0755"
        ),
        CreateStable(
            Platform.MacOS,
            "BepInEx_macos_universal_5.4.23.6.zip",
            "2e711e21f6570da964fce4654b1976500dbe043f61778bce0008500078564638"
        ),
        CreateStable(
            Platform.Linux,
            "BepInEx_linux_x64_5.4.23.6.zip",
            "f8ac87a97fb1ef85451c04ace1fddb7dd7eb0b079c179b989d6bc462c98bd365"
        ),
        CreateBeta(
            Platform.Windows,
            "win-x64",
            "5dce6c73f760933957f477839a6e28e5d359f196a5cef4a5c90f5a7a73fce82c"
        ),
        CreateBeta(
            Platform.MacOS,
            "macos-x64",
            "68af2c22f9c23d343a5cdfade005e64aba6850a4f77824041f8cb78491cbca39"
        ),
        CreateBeta(
            Platform.Linux,
            "linux-x64",
            "9300b9694eb9cbe2b794282e247e06198a26001e729b63021ebecf0c42e82810"
        ),
    ];

    public static BepInExRelease Get(BepInExMajor major, Platform platform) =>
        All.Single(release => release.Major == major && release.Platform == platform);

    private static BepInExRelease CreateStable(Platform platform, string fileName, string sha256) =>
        new(
            BepInExMajor.Stable,
            StableVersion,
            platform,
            fileName,
            new Uri(StableDownloadUrl + fileName),
            sha256
        );

    private static BepInExRelease CreateBeta(Platform platform, string buildTarget, string sha256)
    {
        string fileName = $"BepInEx-Unity.Mono-{buildTarget}-{BetaBuildVersion}.zip";
        return new(
            BepInExMajor.Beta,
            BetaVersion,
            platform,
            fileName,
            new Uri(BetaDownloadUrl + Uri.EscapeDataString(fileName)),
            sha256
        );
    }
}
