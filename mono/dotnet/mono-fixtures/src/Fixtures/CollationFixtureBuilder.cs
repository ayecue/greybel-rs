using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal sealed class CollationFixtureBuilder : IFixtureBuilder
{
    private static readonly string[] Alphabet =
    [
        "a",
        "A",
        "b",
        "B",
        "c",
        "e",
        "s",
        "z",
        "Z",
        "0",
        "1",
        "9",
        " ",
        "-",
        "'",
        "_",
        ".",
        ",",
        "!",
        "~",
        "\u0000",
        "\u0001",
        "\t",
        "\u00ad",
        "\u200b",
        "\ufeff",
        "\u0301",
        "\u0308",
        "\u0327",
        "\u00e9",
        "\u00c9",
        "\u00e8",
        "\u00df",
        "\u00e6",
        "\u00c6",
        "\u0153",
        "\ufb01",
        "\u00f1",
        "\u00fc",
        "\u0131",
        "\u0130",
        "\u03c3",
        "\u03c2",
        "\u03a3",
        "\u0416",
        "\u0436",
        "\u05d0",
        "\u0627",
        "\u0640",
        "\u0e01",
        "\u0e40",
        "\u3042",
        "\u3041",
        "\u30a2",
        "\u30a1",
        "\u30fc",
        "\u309d",
        "\u309e",
        "\u30fd",
        "\u30fe",
        "\u3005",
        "\uff71",
        "\uff70",
        "\uff9e",
        "\u3099",
        "\u304b",
        "\u304c",
        "\u30ab",
        "\u30ac",
        "\u4e00",
        "\u4e8c",
        "\uac00",
        "\uac01",
        "\u1100",
        "\u1161",
        "\uff21",
        "\uff41",
        "\uff10",
        "\u2160",
        "\u2170",
        "\u2460",
        "\u00bd",
        "\u00b2",
        "\u2015",
        "\u2014",
        "\u2013",
        "\u0378",
        "\u0870",
        "\ue000",
        "\uffff",
        "\ufffd",
        "\U0001F600",
        "\U0001D11E",
        "\ud800",
        "\udc00",
        "\ud880",
        "\u02bc",
        "\u02c6",
        "\u1e9b",
    ];

    private static readonly string[] Tables = ["collation.core.bin", "collation.tailoring.bin"];

    private const int RandomTexts = 260;
    private const int SuffixTrials = 6000;

    public string CrateName => "mono-string";

    public void Write(Crate crate)
    {
        WriteTables(crate);
        List<string> texts = GetTexts();
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteArray("texts", texts.Select(Json.FormatUnits));
        json.WriteProperty("compare", Json.FormatString(FormatCompares(texts)));
        json.WriteArray("ends_with", FormatEndsWithRows(texts));
        json.WriteEndObject();
        json.Save(crate.GetFixturePath("collation.json"));
    }

    private static void WriteTables(Crate crate)
    {
        foreach (string name in Tables)
        {
            using Stream table =
                typeof(object).Assembly.GetManifestResourceStream(name)
                ?? throw new InvalidOperationException($"the corlib has no {name} resource");
            OutputFile.Copy(
                table,
                crate.GetPath("src", "collation", name.Substring("collation.".Length))
            );
        }
        var unassignedBits = new byte[0x10000 / 8];
        for (int unit = 0; unit <= 0xFFFF; unit++)
        {
            if (char.GetUnicodeCategory((char)unit) == UnicodeCategory.OtherNotAssigned)
            {
                unassignedBits[unit / 8] |= (byte)(1 << (unit % 8));
            }
        }
        OutputFile.WriteBytes(crate.GetPath("src", "collation", "unassigned.bin"), unassignedBits);
    }

    private static string FormatCompares(List<string> texts)
    {
        var compares = new StringBuilder();
        foreach (string a in texts)
        {
            foreach (string b in texts)
            {
                compares.Append("-0+"[Math.Sign(a.CompareTo(b)) + 1]);
            }
        }
        return compares.ToString();
    }

    private const ulong SuffixSeed = 0xc011a7e;

    private static IEnumerable<string> FormatEndsWithRows(List<string> texts)
    {
        var random = new SplitMix(SuffixSeed);
        for (int trial = 0; trial < SuffixTrials; trial++)
        {
            string text = texts[random.NextInt(texts.Count)];
            string suffix = Mutate(
                random,
                text.Substring(text.Length - random.NextInt(text.Length + 1))
            );
            yield return Json.FormatArray([
                Json.FormatUnits(text),
                Json.FormatUnits(suffix),
                Json.FormatBool(text.EndsWith(suffix)),
            ]);
        }
    }

    private const ulong CollationTextSeed = 0x5047;

    private static List<string> GetTexts()
    {
        var random = new SplitMix(CollationTextSeed);
        var texts = new List<string> { "" };
        while (texts.Count < RandomTexts)
        {
            string text =
                random.NextInt(3) == 0
                    ? CreateRandomText(random, 1 + random.NextInt(6))
                    : Mutate(random, texts[random.NextInt(texts.Count)]);
            texts.Add(text);
        }
        return texts;
    }

    private static string CreateRandomText(SplitMix random, int length)
    {
        var text = new StringBuilder();
        for (int i = 0; i < length; i++)
        {
            text.Append(Alphabet[random.NextInt(Alphabet.Length)]);
        }
        return text.ToString();
    }

    private static string Mutate(SplitMix random, string text)
    {
        int at = random.NextInt(text.Length + 1);
        string piece = Alphabet[random.NextInt(Alphabet.Length)];
        return random.NextInt(3) switch
        {
            0 => text.Insert(at, piece),
            1 => at < text.Length ? text.Remove(at, 1) : text + piece,
            _ => at < text.Length ? text.Remove(at, 1).Insert(at, piece) : text + piece,
        };
    }
}
