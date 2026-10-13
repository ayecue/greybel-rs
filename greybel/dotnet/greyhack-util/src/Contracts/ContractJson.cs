using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace GreyHackUtil.Contracts;

public static class ContractJson
{
    private static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    public static ContractFile Load(string path)
    {
        try
        {
            return Parse(File.ReadAllText(path), path);
        }
        catch (IOException error)
        {
            throw new ContractException(error.Message);
        }
    }

    public static ContractFile Parse(string json, string path)
    {
        try
        {
            return JsonSerializer.Deserialize<ContractFile>(json, Options)
                ?? throw new ContractException($"{path} is empty");
        }
        catch (JsonException error)
        {
            throw new ContractException($"{path}: {error.Message}");
        }
    }

    public static string Serialize(ContractFile contract)
    {
        var builder = new StringBuilder();
        Write(JsonSerializer.SerializeToNode(contract, Options)!, builder, "");
        return builder.Append('\n').ToString();
    }

    private static void Write(JsonNode node, StringBuilder builder, string indent)
    {
        string inner = indent + "  ";
        switch (node)
        {
            case JsonObject obj when obj.All(p => p.Value is JsonValue or null):
                builder
                    .Append("{ ")
                    .AppendJoin(", ", obj.Select(p => $"{Quote(p.Key)}: {FormatScalar(p.Value)}"))
                    .Append(" }");
                break;
            case JsonObject obj:
                builder.Append("{\n");
                int index = 0;
                foreach (var (key, value) in obj)
                {
                    builder.Append(inner).Append(Quote(key)).Append(": ");
                    Write(value!, builder, inner);
                    builder.Append(++index < obj.Count ? ",\n" : "\n");
                }
                builder.Append(indent).Append('}');
                break;
            case JsonArray array when array.All(v => v is JsonValue or null):
                builder.Append('[').AppendJoin(", ", array.Select(FormatScalar)).Append(']');
                break;
            case JsonArray array:
                builder.Append("[\n");
                for (int i = 0; i < array.Count; i++)
                {
                    builder.Append(inner);
                    Write(array[i]!, builder, inner);
                    builder.Append(i + 1 < array.Count ? ",\n" : "\n");
                }
                builder.Append(indent).Append(']');
                break;
            default:
                builder.Append(FormatScalar(node));
                break;
        }
    }

    private static string Quote(string text) => JsonSerializer.Serialize(text, Options);

    private static string FormatScalar(JsonNode? node) => node?.ToJsonString(Options) ?? "null";
}
