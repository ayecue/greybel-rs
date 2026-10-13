using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal sealed class StringFixtureBuilder : IFixtureBuilder
{
    private static readonly string[] Texts =
    [
        "",
        "a",
        "abc",
        "Hello, World!",
        "\u00e9",
        "a\u00e9\U0001F600b",
        "\U0001F600",
        "\U0001F600\U0001F601",
        "\u00df",
        "\u0130",
        "\u0131",
        "\u03a3\u0391\u03a3",
        "\u01c5",
        "\u00b5",
        "\u017f",
        "\ufb00",
        "\u0149",
        "stra\u00dfe",
        "\u00c6ble",
        "x\u0000y",
        "\u0000",
        "\u0000abc",
        "ab\u0000",
        "\uff61",
        "\ue000",
        "\uffff",
        "\U0010ffff",
        "\u2028",
        "\u10a0",
        "\u2c00",
        "\u24b6",
        "\u2160",
        "\u1e9e",
        "\u0587",
        "\u1f80",
        "\u0345",
        "\u03c2",
        "\u03d0",
        "\u1e9b",
        "\u0390",
        "\u1fb3",
        "\u01f0",
        "\u0587\u0130i",
        "MiXeD cAsE 123",
        "\t\n ",
        "\ud7ff",
        "\uabcd\u1234",
        "\u0130stanbul",
        "I\u0307",
    ];

    private static readonly string[] Needles =
    [
        "",
        "a",
        "b",
        "ab",
        "\u00e9",
        "\U0001F600",
        "\u0000",
        "c",
        "abc",
        "x",
    ];

    private const string RandomAlphabet =
        "aAzZ09 \u00e9\u00c9\u00df\u0130\u0131\u03a3\u03c3\u03c2\u0416\u0436\u10a0\u2c00\uff21\uff41\ue000\uffff";

    public string CrateName => "mono-string";

    public void Write(Crate crate)
    {
        List<string> texts = Texts.Concat(CreateRandomTexts()).ToList();
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteArray("texts", texts.Select(FormatTextRow));
        json.WriteArray("compare", FormatCompareRows(texts.Take(60).ToList()));
        json.WriteArray("substring", FormatSubstringRows(texts.Where(text => text.Length <= 8)));
        json.WriteArray("search", FormatSearchRows(texts.Take(40)));
        json.WriteEndObject();
        json.Save(crate.GetFixturePath("string.json"));
    }

    private static string FormatTextRow(string text)
    {
        TextInfo info = CultureInfo.InvariantCulture.TextInfo;
        return Json.FormatArray([
            Json.FormatString(text),
            Json.FormatInt(text.Length),
            Json.FormatInt(text.GetHashCode()),
            Json.FormatString(info.ToUpper(text)),
            Json.FormatString(info.ToLower(text)),
        ]);
    }

    private static IEnumerable<string> FormatCompareRows(List<string> texts) =>
        from a in texts
        from b in texts
        select Json.FormatArray([
            Json.FormatString(a),
            Json.FormatString(b),
            Json.FormatInt(Math.Sign(string.Compare(a, b, StringComparison.Ordinal))),
        ]);

    private static IEnumerable<string> FormatSubstringRows(IEnumerable<string> texts) =>
        from text in texts
        from start in Enumerable.Range(0, text.Length + 1)
        from length in Enumerable.Range(0, text.Length - start + 1)
        select Json.FormatArray([
            Json.FormatString(text),
            Json.FormatInt(start),
            Json.FormatInt(length),
            Json.FormatUnits(text.Substring(start, length)),
        ]);

    private static IEnumerable<string> FormatSearchRows(IEnumerable<string> texts) =>
        from text in texts
        from needle in Needles
        select Json.FormatArray([
            Json.FormatString(text),
            Json.FormatString(needle),
            Json.FormatInt(text.IndexOf(needle, StringComparison.Ordinal)),
            Json.FormatInt(text.LastIndexOf(needle, StringComparison.Ordinal)),
        ]);

    private const ulong TextSeed = 0x57a1;

    private static IEnumerable<string> CreateRandomTexts()
    {
        var random = new SplitMix(TextSeed);
        for (int i = 0; i < 40; i++)
        {
            var text = new StringBuilder();
            int length = random.NextInt(10);
            for (int j = 0; j < length; j++)
            {
                if (random.NextInt(6) == 0)
                {
                    text.Append(char.ConvertFromUtf32(0x10000 + random.NextInt(0xFFFFF)));
                }
                else
                {
                    text.Append(RandomAlphabet[random.NextInt(RandomAlphabet.Length)]);
                }
            }
            yield return text.ToString();
        }
    }
}
