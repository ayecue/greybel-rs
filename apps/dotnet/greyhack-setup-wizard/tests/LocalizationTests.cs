using System.Collections;
using System.Globalization;
using System.Resources;
using System.Text.RegularExpressions;
using GreyHackSetupWizard.Gui.Resources;
using GreyHackSetupWizard.Tests.Support;
using static GreyHackSetupWizard.Tests.Support.LocalizationSetup;

namespace GreyHackSetupWizard.Tests;

public class LocalizationTests
{
    [Fact]
    public void ShouldFallBackToEnglishForALanguageWithoutTexts()
    {
        CultureInfo culture = CultureInfo.CurrentUICulture;
        try
        {
            Localization.Apply("de-DE");

            Assert.Equal("de-DE", CultureInfo.CurrentUICulture.Name);
            Assert.Equal("Installation complete.", WindowText.OutcomeComplete);
        }
        finally
        {
            CultureInfo.CurrentUICulture = culture;
            CultureInfo.DefaultThreadCurrentUICulture = null;
        }
    }

    [Fact]
    public void ShouldNumberEveryTextsPlaceholdersFromZero()
    {
        ResourceSet texts = WindowText.ResourceManager.GetResourceSet(
            CultureInfo.InvariantCulture,
            createIfNotExists: true,
            tryParents: true
        )!;
        var gaps = new List<string>();

        foreach (DictionaryEntry text in texts)
        {
            int[] indexes = GetPlaceholderRegex()
                .Matches((string)text.Value!)
                .Select(match => int.Parse(match.Groups[1].Value))
                .Distinct()
                .Order()
                .ToArray();
            if (!indexes.SequenceEqual(Enumerable.Range(0, indexes.Length)))
            {
                gaps.Add((string)text.Key);
            }
        }

        Assert.Empty(gaps);
    }
}
