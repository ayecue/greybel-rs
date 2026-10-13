#nullable enable
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace GreyHackMessageHook.Game;

public sealed class RawMessageReader
{
    private readonly JObject json;
    private int cursor;

    internal RawMessageReader(JObject json)
    {
        this.json = json;
    }

    public bool HasNext => json["params"] is JArray values && cursor < values.Count;

    public JToken Next()
    {
        if (json["params"] is not JArray values || cursor >= values.Count)
        {
            throw new JsonException($"message {json["method"]} has no parameter {cursor}");
        }
        return values[cursor++];
    }
}
