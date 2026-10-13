using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard;

public enum Platform
{
    Windows,
    MacOS,
    Linux,
}

public static class PlatformDetector
{
    public static Platform Current =>
        OperatingSystem.IsWindows() ? Platform.Windows
        : OperatingSystem.IsMacOS() ? Platform.MacOS
        : OperatingSystem.IsLinux() ? Platform.Linux
        : throw new SetupException(Messages.PlatformUnsupported);
}
