using System.Globalization;
using GreyHackSetupWizard.Resources;

namespace GreyHackSetupWizard;

public static class Localization
{
    public const string Option = "--language";

    public static void Apply(string? language)
    {
        if (string.IsNullOrEmpty(language))
        {
            return;
        }
        CultureInfo culture;
        try
        {
            culture = CultureInfo.GetCultureInfo(language, predefinedOnly: true);
        }
        catch (CultureNotFoundException)
        {
            throw new SetupException(Format(Messages.LanguageUnknown, language));
        }
        CultureInfo.DefaultThreadCurrentUICulture = culture;
        CultureInfo.CurrentUICulture = culture;
    }

    public static string? FindLanguage(IReadOnlyList<string> arguments)
    {
        for (int index = 0; index < arguments.Count; index++)
        {
            if (arguments[index] == Option && index + 1 < arguments.Count)
            {
                return arguments[index + 1];
            }
            if (arguments[index].StartsWith(Option + "=", StringComparison.Ordinal))
            {
                return arguments[index][(Option.Length + 1)..];
            }
        }
        return null;
    }

    public static string Format(string text, params object?[] values) =>
        string.Format(CultureInfo.CurrentCulture, text, values);
}
