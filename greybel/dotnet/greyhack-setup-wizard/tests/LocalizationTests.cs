using System.Collections;
using System.Globalization;
using System.Resources;
using System.Text.RegularExpressions;
using GreyHackSetupWizard.Resources;
using GreyHackSetupWizard.Tests.Support;
using static GreyHackSetupWizard.Tests.Support.LocalizationSetup;

namespace GreyHackSetupWizard.Tests;

public class LocalizationTests
{
    [Theory]
    [InlineData(new[] { "locate", "--language", "de" }, "de")]
    [InlineData(new[] { "--language=en-GB", "locate" }, "en-GB")]
    [InlineData(new[] { "locate" }, null)]
    public void ShouldFindTheLanguageInTheArguments(string[] arguments, string? language)
    {
        string? found = Localization.FindLanguage(arguments);

        Assert.Equal(language, found);
    }

    [Fact]
    public void ShouldRejectAnUnknownLanguage()
    {
        var error = Assert.Throws<SetupException>(() => Localization.Apply("xx-unknown"));

        Assert.Equal("xx-unknown isn't a known language.", error.Message);
    }

    [Fact]
    public void ShouldFallBackToEnglishForALanguageWithoutTexts()
    {
        CultureInfo culture = CultureInfo.CurrentUICulture;
        try
        {
            Localization.Apply("de-DE");

            Assert.Equal("de-DE", CultureInfo.CurrentUICulture.Name);
            Assert.Equal("BepInEx isn't installed. Install it first.", Messages.BepInExMissing);
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
        ResourceSet texts = Messages.ResourceManager.GetResourceSet(
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
