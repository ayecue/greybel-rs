using System.Text.Json.Serialization;

namespace GreyHackUtil.Contracts;

[JsonConverter(typeof(JsonStringEnumConverter<ValueKind>))]
public enum ValueKind
{
    [JsonStringEnumMemberName("string")]
    String,

    [JsonStringEnumMemberName("int32")]
    Int32,

    [JsonStringEnumMemberName("int64")]
    Int64,

    [JsonStringEnumMemberName("uint64")]
    UInt64,

    [JsonStringEnumMemberName("bool")]
    Bool,

    [JsonStringEnumMemberName("float32")]
    Float32,

    [JsonStringEnumMemberName("float64")]
    Float64,

    [JsonStringEnumMemberName("bytes")]
    Bytes,

    [JsonStringEnumMemberName("int32[]")]
    Int32Array,

    [JsonStringEnumMemberName("string[]")]
    StringList,

    [JsonStringEnumMemberName("enum")]
    Enum,

    [JsonStringEnumMemberName("struct")]
    Struct,
}
