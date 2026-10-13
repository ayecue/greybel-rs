using System.Text.Json.Serialization;

namespace GreyHackUtil.Contracts;

public sealed class ContractFile
{
    public string? GameVersion { get; set; }

    public List<FieldType>? FieldTypes { get; set; }

    public List<MessageContract> Server { get; set; } = [];

    public List<MessageContract> Client { get; set; } = [];
}

public sealed class FieldType
{
    public string Key { get; set; } = "";

    public string Reader { get; set; } = "";

    public string ClrType { get; set; } = "";

    public string TypeName { get; set; } = "";

    public bool Nullable { get; set; }

    public ValueKind Type { get; set; }

    public Dictionary<string, long>? Members { get; set; }

    public List<StructField>? Fields { get; set; }

    [JsonIgnore]
    public string FieldName { get; set; } = "";

    [JsonIgnore]
    public string ShortClrType =>
        TypeName.Length > 0 ? TypeName : ClrType[(ClrType.LastIndexOf('.') + 1)..];
}

public sealed class StructField
{
    public string Name { get; set; } = "";

    public ValueKind Type { get; set; }
}

public sealed class MessageContract
{
    public string Name { get; set; } = "";

    public int Id { get; set; }

    public string? Handler { get; set; }

    public string? Window { get; set; }

    public List<string>? Senders { get; set; }

    public List<FieldContract>? Fields { get; set; }
}

public sealed class FieldContract
{
    public string Name { get; set; } = "";

    public string FieldType { get; set; } = "";

    public bool Optional { get; set; }
}
