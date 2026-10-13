using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal sealed class DictionaryFixtureBuilder : IFixtureBuilder
{
    private static readonly int[] KeyRanges = [4, 16, 64, 512];

    private const int SeriesPerRange = 4;
    private const int Steps = 150;

    public string CrateName => "mono-dictionary";

    public void Write(Crate crate)
    {
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteStartArray("series");
        for (int series = 0; series < KeyRanges.Length * SeriesPerRange; series++)
        {
            json.WriteStartObject();
            json.WriteArray("steps", FormatSteps(series, KeyRanges[series / SeriesPerRange]));
            json.WriteEndObject();
        }
        json.WriteEndArray();
        json.WriteEndObject();
        json.Save(crate.GetFixturePath("dictionary.json"));
    }

    private static IEnumerable<string> FormatSteps(int seed, int keys)
    {
        var random = new Random(seed);
        var dictionary = new Dictionary<int, int>();
        for (int step = 0; step < Steps; step++)
        {
            int roll = random.Next(100);
            int key = random.Next(keys);
            string change;
            if (roll < 2)
            {
                dictionary.Clear();
                change = Json.FormatArray([Json.FormatString("clear")]);
            }
            else if (roll < 40)
            {
                bool hadKey = dictionary.Remove(key);
                change = Json.FormatArray([
                    Json.FormatString("remove"),
                    Json.FormatInt(key),
                    Json.FormatBool(hadKey),
                ]);
            }
            else
            {
                dictionary[key] = step;
                change = Json.FormatArray([
                    Json.FormatString("set"),
                    Json.FormatInt(key),
                    Json.FormatInt(step),
                ]);
            }
            string entries = Json.FormatArray(
                dictionary.Select(entry =>
                    Json.FormatArray([Json.FormatInt(entry.Key), Json.FormatInt(entry.Value)])
                )
            );
            yield return Json.FormatObject(("change", change), ("entries", entries));
        }
    }
}
