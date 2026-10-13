using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

internal sealed class ValueTypeCatalog
{
    private readonly Dictionary<string, FieldType> byClrType = [];

    public IReadOnlyList<FieldType> Types => byClrType.Values.ToList();

    public FieldType Read(IType type)
    {
        if (byClrType.TryGetValue(type.ReflectionName, out FieldType? catalogEntry))
        {
            return catalogEntry;
        }
        var fieldType = new FieldType
        {
            Key = type.ReflectionName,
            ClrType = type.ReflectionName,
            TypeName = type.GetDefinition()?.Name ?? type.Name,
        };
        Describe(type, fieldType);
        byClrType.Add(type.ReflectionName, fieldType);
        return fieldType;
    }

    private void Describe(IType type, FieldType fieldType)
    {
        if (type is ParameterizedType { Name: "Nullable", TypeArguments.Count: 1 } nullable)
        {
            fieldType.Nullable = true;
            fieldType.TypeName =
                nullable.TypeArguments[0].GetDefinition()?.Name ?? nullable.TypeArguments[0].Name;
            Describe(nullable.TypeArguments[0], fieldType);
            return;
        }
        if (type.Kind == TypeKind.Enum)
        {
            ITypeDefinition definition = type.GetDefinition()!;
            if (
                definition.EnumUnderlyingType?.GetDefinition()?.KnownTypeCode != KnownTypeCode.Int32
            )
            {
                throw new ContractException($"enum {type.FullName} is not int32 based");
            }
            fieldType.Type = ValueKind.Enum;
            fieldType.Members = definition
                .Fields.Where(f => f.IsConst)
                .ToDictionary(f => f.Name, f => Convert.ToInt64(f.GetConstantValue()));
            return;
        }
        if (
            type.Kind == TypeKind.Struct
            && type.GetDefinition()?.KnownTypeCode == KnownTypeCode.None
        )
        {
            fieldType.Type = ValueKind.Struct;
            fieldType.Fields = type.GetFields(f =>
                    !f.IsStatic && f.Accessibility == Accessibility.Public
                )
                .Select(f => new StructField { Name = f.Name, Type = ToValueKind(f.Type) })
                .ToList();
            return;
        }
        fieldType.Type = ToValueKind(type);
    }

    private static ValueKind ToValueKind(IType type)
    {
        if (type is ArrayType { Dimensions: 1 } array)
        {
            return array.ElementType.GetDefinition()?.KnownTypeCode switch
            {
                KnownTypeCode.Byte => ValueKind.Bytes,
                KnownTypeCode.Int32 => ValueKind.Int32Array,
                _ => throw new ContractException($"unsupported value type {type.FullName}"),
            };
        }
        if (type is ParameterizedType { Name: "List", TypeArguments.Count: 1 } list)
        {
            return list.TypeArguments[0].GetDefinition()?.KnownTypeCode switch
            {
                KnownTypeCode.String => ValueKind.StringList,
                KnownTypeCode.Int32 => ValueKind.Int32Array,
                _ => throw new ContractException($"unsupported value type {type.FullName}"),
            };
        }
        return type.GetDefinition()?.KnownTypeCode switch
        {
            KnownTypeCode.String => ValueKind.String,
            KnownTypeCode.Int32 => ValueKind.Int32,
            KnownTypeCode.Int64 => ValueKind.Int64,
            KnownTypeCode.UInt64 => ValueKind.UInt64,
            KnownTypeCode.Boolean => ValueKind.Bool,
            KnownTypeCode.Single => ValueKind.Float32,
            KnownTypeCode.Double => ValueKind.Float64,
            _ => throw new ContractException($"unsupported value type {type.FullName}"),
        };
    }
}

internal static class FieldTypeReader
{
    public const string MessageType = "NetworkMessages.RpcMessage";
    public const string ParameterReader = "GetParam";
}
