using System.Text.RegularExpressions;

namespace GreyHackSetupWizard.Tests.Support;

internal static partial class LocalizationSetup
{
    [GeneratedRegex(@"\{(\d+)[^}]*\}")]
    internal static partial Regex GetPlaceholderRegex();
}
