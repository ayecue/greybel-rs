using System.Text.RegularExpressions;
using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.Semantics;
using ICSharpCode.Decompiler.TypeSystem;
using VariableKind = ICSharpCode.Decompiler.IL.VariableKind;

namespace GreyHackUtil.Extraction;

internal sealed record SentValue(
    string FieldType,
    string? Name,
    bool WindowPid,
    bool OwnPid,
    string? Parameter
);

internal sealed record SentLayout(string Where, IMethod? Method, IReadOnlyList<SentValue> Values);

internal sealed partial class SenderReader
{
    private readonly GameAssembly game;
    private readonly ValueTypeCatalog types;
    private readonly WindowPidReader windowPids;

    public SenderReader(GameAssembly game, ValueTypeCatalog types)
    {
        this.game = game;
        this.types = types;
        windowPids = new WindowPidReader(game);
    }

    public Dictionary<(string IdType, string Message), List<SentLayout>> Read(
        IReadOnlyDictionary<string, string> messageTypes,
        ICollection<string> warnings
    )
    {
        var layouts = new Dictionary<(string, string), List<SentLayout>>();
        var constructors = messageTypes
            .Keys.SelectMany(type =>
                game.FindType(type)
                    .Methods.Where(m => m.IsConstructor && m.Parameters.Count is 1 or 2)
            )
            .ToList();
        foreach (ITypeDefinition type in game.FindCallerTypes(constructors, GameAssembly.NewObject))
        {
            SyntaxTree tree = game.DecompileType(type);
            foreach (
                ObjectCreateExpression creation in tree.Descendants.OfType<ObjectCreateExpression>()
            )
            {
                if (
                    creation.GetResolveResult()
                        is not InvocationResolveResult { Member: IMethod constructor }
                    || !constructors.Contains((IMethod)constructor.MemberDefinition)
                    || creation.Arguments.FirstOrDefault()?.GetResolveResult()
                        is not MemberResolveResult { Member: IField { IsConst: true } id }
                )
                {
                    continue;
                }
                string where =
                    $"{type.Name}.{(creation.Ancestors.OfType<EntityDeclaration>().FirstOrDefault()?.Name ?? "?")}";
                var values = new List<SentValue>();
                AstNode declaration = creation.Ancestors.OfType<EntityDeclaration>().First();
                foreach (Expression argument in creation.Arguments.Skip(1))
                {
                    IType valueType =
                        argument.GetResolveResult()?.Type
                        ?? throw new ContractException(
                            $"{where}: cannot determine the type sent by {argument}"
                        );
                    string fieldType =
                        valueType.FullName == "null" ? "" : types.Read(valueType).Key;
                    values.Add(
                        new SentValue(
                            fieldType,
                            GetValueName(argument, declaration),
                            windowPids.HasPid(argument, declaration),
                            windowPids.HasOwnPid(argument, declaration),
                            GetParameter(argument, declaration)
                        )
                    );
                }
                var key = (id.DeclaringType.FullName, id.Name);
                if (!layouts.TryGetValue(key, out var messageLayouts))
                {
                    layouts[key] = messageLayouts = [];
                }
                messageLayouts.Add(
                    new SentLayout(where, declaration.GetSymbol() as IMethod, values)
                );
            }
        }
        return layouts;
    }

    public void Describe(
        MessageContract message,
        IReadOnlyList<SentLayout> layouts,
        ICollection<string> warnings
    )
    {
        if (message.Fields != null)
        {
            foreach (SentLayout layout in layouts.Where(layout => !Covers(layout, message.Fields)))
            {
                warnings.Add(
                    $"{layout.Where} sends {message.Name} with less than its receiver reads"
                );
            }
        }
        else
        {
            AssignTypesFromSenders(message, layouts, warnings);
        }
        if (message.Fields != null && message.Window == null)
        {
            message.Window = WindowReader.FromSenders(message, message.Fields, layouts, warnings);
        }
    }

    private void AssignTypesFromSenders(
        MessageContract message,
        IReadOnlyList<SentLayout> layouts,
        ICollection<string> warnings
    )
    {
        if (layouts.Count == 0)
        {
            return;
        }
        var shapes = layouts
            .Select(layout => string.Join(",", layout.Values.Select(value => value.FieldType)))
            .Distinct()
            .ToList();
        if (shapes.Count > 1)
        {
            warnings.Add(
                $"{message.Name}: its senders give it {shapes.Count} layouts, so it has none"
            );
            return;
        }
        IReadOnlyList<SentValue> values = layouts[0].Values;
        if (values.Any(value => value.FieldType.Length == 0))
        {
            warnings.Add(
                $"{message.Name}: its senders pass an untyped null, so it has no contract layout"
            );
            return;
        }
        string?[] names =
            GetMethodNames(message.Name, values) ?? values.Select(value => value.Name).ToArray();
        var fields = new List<FieldContract>();
        for (int index = 0; index < values.Count; index++)
        {
            string name = string.IsNullOrEmpty(names[index])
                ? GetStem(values[index].FieldType)
                : names[index]!;
            string unique = name;
            for (int suffix = 2; fields.Any(field => field.Name == unique); suffix++)
            {
                unique = name + suffix;
            }
            fields.Add(new FieldContract { Name = unique, FieldType = values[index].FieldType });
        }
        message.Fields = fields;
        message.Senders = layouts.Select(layout => layout.Where).Distinct().ToList();
    }

    private static bool Covers(SentLayout layout, IReadOnlyList<FieldContract> fields)
    {
        int mandatoryCount = fields.TakeWhile(field => !field.Optional).Count();
        return layout.Values.Count >= mandatoryCount
            && layout.Values.Count <= fields.Count
            && layout
                .Values.Select((value, index) => value.FieldType == fields[index].FieldType)
                .All(fits => fits);
    }

    private string?[]? GetMethodNames(string message, IReadOnlyList<SentValue> values)
    {
        var candidates = game
            .Module.TypeDefinitions.SelectMany(type => type.Methods)
            .Where(method =>
                method.Name == message
                && method.Parameters.Count == values.Count
                && method
                    .Parameters.Select(
                        (parameter, index) =>
                            parameter.Type.ReflectionName == values[index].FieldType
                    )
                    .All(fits => fits)
            )
            .ToList();
        return candidates.Count == 1
            ? candidates[0].Parameters.Select(parameter => (string?)parameter.Name).ToArray()
            : null;
    }

    internal static string GetStem(string fieldType) =>
        fieldType switch
        {
            "System.String" => "string",
            "System.Int32" => "int",
            "System.Boolean" => "bool",
            "System.Byte[]" => "bytes",
            _ => fieldType.EndsWith('s') ? fieldType[..^1] : fieldType,
        };

    internal static string? GetParameter(Expression value, AstNode scope)
    {
        while (value is CastExpression or ParenthesizedExpression)
        {
            value = value is CastExpression cast
                ? cast.Expression
                : ((ParenthesizedExpression)value).Expression;
        }
        string? name = value.GetResolveResult() switch
        {
            ILVariableResolveResult
            {
                Variable: { Kind: VariableKind.Parameter, Name: var parameter }
            } => parameter,
            LocalResolveResult { Variable: IParameter parameter } => parameter.Name,
            _ => null,
        };
        return
            name != null
            && scope.GetSymbol() is IMethod method
            && method.Parameters.Any(own => own.Name == name)
            && !value
                .Ancestors.TakeWhile(node => node != scope)
                .Any(node =>
                    node switch
                    {
                        LambdaExpression lambda => lambda.Parameters.Any(p => p.Name == name),
                        AnonymousMethodExpression anonymous => anonymous.Parameters.Any(p =>
                            p.Name == name
                        ),
                        _ => false,
                    }
                )
            ? name
            : null;
    }

    internal static bool IsGetter(string name) =>
        name.StartsWith("Get") && name.Length > 3 && char.IsUpper(name[3]);

    internal static string GetGetterName(string name)
    {
        string suffix = name[3..];
        int capitals = suffix.TakeWhile(char.IsUpper).Count();
        int lowerCount = capitals > 1 && capitals < suffix.Length ? capitals - 1 : capitals;
        return suffix[..lowerCount].ToLowerInvariant() + suffix[lowerCount..];
    }

    [GeneratedRegex(@"^(text|num|flag|array|obj|value|item|list|result|str|b|i|j|k)\d*$")]
    private static partial Regex GetDefaultLocalNameRegex();

    private static string? GetValueName(Expression value, AstNode scope)
    {
        while (value is CastExpression or ParenthesizedExpression)
        {
            value = value is CastExpression cast
                ? cast.Expression
                : ((ParenthesizedExpression)value).Expression;
        }
        switch (value)
        {
            case IdentifierExpression identifier:
                if (
                    identifier.GetResolveResult() is LocalResolveResult
                    {
                        Variable: IParameter parameter
                    }
                )
                {
                    return parameter.Name;
                }
                if (identifier.GetResolveResult() is MemberResolveResult)
                {
                    return identifier.Identifier;
                }
                var initializers = scope
                    .Descendants.OfType<VariableInitializer>()
                    .Where(v => v.Name == identifier.Identifier)
                    .ToList();
                string? initial =
                    initializers.Count == 1 && !initializers[0].Initializer.IsNull
                        ? GetValueName(initializers[0].Initializer, scope)
                        : null;
                return initial
                    ?? (
                        GetDefaultLocalNameRegex().IsMatch(identifier.Identifier)
                            ? null
                            : identifier.Identifier
                    );
            case MemberReferenceExpression member:
                return
                    member.GetResolveResult() is MemberResolveResult { Member: var reference }
                    && reference.ParentModule?.IsMainModule != true
                    ? GetValueName(member.Target, scope)
                    : member.MemberName;
            case InvocationExpression { Target: MemberReferenceExpression { MemberName: var name } }
                when IsGetter(name):
                return GetGetterName(name);
            case InvocationExpression { Target: IdentifierExpression { Identifier: var name } }
                when IsGetter(name):
                return GetGetterName(name);
            case InvocationExpression { Arguments.Count: 1 } call:
                return GetValueName(call.Arguments.Single(), scope);
            case InvocationExpression { Target: MemberReferenceExpression { Target: var target } }:
                return GetValueName(target, scope);
            default:
                return null;
        }
    }
}
