#nullable enable
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace GreyHackMessageHook.Game;

public sealed class RawMessage
{
    private readonly JObject json;

    public int Id => json.Value<int>("method");

    public RawMessage(int id)
    {
        json = new JObject { ["method"] = id, ["params"] = new JArray() };
    }

    private RawMessage(JObject json)
    {
        this.json = json;
    }

    public static RawMessage Parse(string text) => new(JObject.Parse(text));

    public T ToObject<T>() => json.ToObject<T>()!;

    public void Add<T>(T value)
    {
        if (json["params"] is not JArray values)
        {
            values = new JArray();
            json["params"] = values;
        }
        values.Add(value == null ? JValue.CreateNull() : JToken.FromObject(value));
    }

    public RawMessageReader GetReader() => new(json);

    public string ToJson() => json.ToString(Formatting.None);

    public override string ToString() => ToJson();
}
