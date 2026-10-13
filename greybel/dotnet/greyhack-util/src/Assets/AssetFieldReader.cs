using System.Text.Json.Nodes;
using AssetsTools.NET;

namespace GreyHackUtil.Assets;

/// <summary>Reads fields from serialized game assets.</summary>
internal static class AssetFieldReader
{
    public static JsonObject ExtractNumbers(AssetTypeValueField structure)
    {
        var numbers = new JsonObject();
        foreach (var field in structure.Children)
        {
            if (ExtractNumber(field) is { } number)
            {
                numbers[
                    field.FieldName.StartsWith("m_", StringComparison.Ordinal)
                        ? field.FieldName[2..]
                        : field.FieldName
                ] = number;
            }
        }
        return numbers;
    }

    public static JsonNode? ExtractNumber(AssetTypeValueField field) =>
        field.Value?.ValueType switch
        {
            AssetValueType.Float => JsonValue.Create(field.AsFloat),
            AssetValueType.Double => JsonValue.Create(field.AsDouble),
            AssetValueType.Bool => JsonValue.Create(field.AsBool ? 1 : 0),
            AssetValueType.Int8 or AssetValueType.Int16 or AssetValueType.Int32 => JsonValue.Create(
                field.AsInt
            ),
            AssetValueType.UInt8 or AssetValueType.UInt16 or AssetValueType.UInt32 =>
                JsonValue.Create(field.AsUInt),
            AssetValueType.Int64 => JsonValue.Create(field.AsLong),
            AssetValueType.UInt64 => JsonValue.Create(field.AsULong),
            _ => null,
        };

    public static IEnumerable<AssetTypeValueField> GetArray(
        AssetTypeValueField structure,
        string name
    )
    {
        AssetTypeValueField field = structure[name];
        return field.IsDummy ? [] : field["Array"].Children;
    }

    public static GameAsset Require(GameAsset? asset, string what) =>
        asset ?? throw new InvalidDataException($"{what} is missing");
}
