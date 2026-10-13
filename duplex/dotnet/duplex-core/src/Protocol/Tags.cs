#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;

namespace Duplex;

public sealed class Tag
{
    public Tag(string name, object? value = null)
    {
        Name = name;
        Value = value;
    }

    public string Name { get; }

    public object? Value { get; }

    public static Tag? Find(IReadOnlyList<Tag> tags, string name) =>
        tags.FirstOrDefault(tag => tag.Name == name);

    public static IEnumerable<Tag> FindAll(IReadOnlyList<Tag> tags, string name) =>
        tags.Where(tag => tag.Name == name);

    public override string ToString() => Value == null ? $"@{Name}" : $"@{Name}({Value})";
}

public sealed class FieldType
{
    private readonly Func<object, object?> get;

    public FieldType(string name, Type type, IReadOnlyList<Tag> tags, Func<object, object?> get)
    {
        Name = name;
        Type = type;
        Tags = tags;
        this.get = get;
    }

    public string Name { get; }

    public Type Type { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public object? Get(object holder) => get(holder);

    public static FieldType? FindWithTag(IReadOnlyList<FieldType> fields, string tag) =>
        fields.FirstOrDefault(field => Tag.Find(field.Tags, tag) != null);

    public override string ToString() => Name;
}

public sealed class StructType
{
    private readonly Func<object?[], object> create;

    public StructType(
        string name,
        IReadOnlyList<Tag> tags,
        IReadOnlyList<FieldType> fields,
        Func<object?[], object> create
    )
    {
        Name = name;
        Tags = tags;
        Fields = fields;
        this.create = create;
    }

    public string Name { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public IReadOnlyList<FieldType> Fields { get; }

    public object Create(object?[] values) => create(values);

    public override string ToString() => Name;
}

public sealed class UnionType
{
    public UnionType(string name, IReadOnlyList<Tag> tags, IReadOnlyList<VariantType> variants)
    {
        Name = name;
        Tags = tags;
        Variants = variants;
    }

    public string Name { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public IReadOnlyList<VariantType> Variants { get; }

    public VariantType GetVariant(object value) =>
        Variants.First(variant => variant.Type == value.GetType());

    public override string ToString() => Name;
}

public sealed class VariantType
{
    public VariantType(
        string name,
        Type type,
        IReadOnlyList<Tag> tags,
        IReadOnlyList<FieldType> fields
    )
    {
        Name = name;
        Type = type;
        Tags = tags;
        Fields = fields;
    }

    public string Name { get; }

    public Type Type { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public IReadOnlyList<FieldType> Fields { get; }

    public override string ToString() => Name;
}

public sealed class EnumType
{
    public EnumType(string name, IReadOnlyList<Tag> tags, IReadOnlyList<MemberType> members)
    {
        Name = name;
        Tags = tags;
        Members = members;
    }

    public string Name { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public IReadOnlyList<MemberType> Members { get; }

    public MemberType? GetMember(uint value) =>
        Members.FirstOrDefault(member => member.Value == value);

    public override string ToString() => Name;
}

public sealed class MemberType
{
    public MemberType(string name, uint value, IReadOnlyList<Tag> tags)
    {
        Name = name;
        Value = value;
        Tags = tags;
    }

    public string Name { get; }

    public uint Value { get; }

    public IReadOnlyList<Tag> Tags { get; }

    public override string ToString() => Name;
}
