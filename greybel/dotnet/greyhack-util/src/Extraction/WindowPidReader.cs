using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Resolver;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.IL;
using ICSharpCode.Decompiler.Semantics;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

internal sealed class WindowPidReader(GameAssembly game)
{
    private const string WindowType = "Ventana";
    private const string OwnPid = "GetPID";

    private readonly Dictionary<ISymbol, bool> members = [];
    private readonly Dictionary<(ISymbol, int), bool> parameters = [];
    private readonly Dictionary<ITypeDefinition, SyntaxTree> types = [];
    private HashSet<ISymbol>? ownFields;

    public bool HasPid(Expression value, AstNode scope)
    {
        while (value is CastExpression or ParenthesizedExpression)
        {
            value = value is CastExpression cast
                ? cast.Expression
                : ((ParenthesizedExpression)value).Expression;
        }
        switch (value.GetResolveResult())
        {
            case InvocationResolveResult { Member: IMethod method }
                when value is InvocationExpression:
                return ReturnsPid(method);
            case MemberResolveResult { Member: IField field }:
                return StoresPid(field);
            case MemberResolveResult { Member: IProperty { Getter: IMethod getter } }:
                return ReturnsPid(getter);
            case ILVariableResolveResult { Variable.Kind: VariableKind.Parameter }
                when SenderReader.GetParameter(value, scope) is string parameter:
                return IsPidPassed(parameter, scope);
            case ILVariableResolveResult { Variable.Kind: not VariableKind.Parameter }
                when value is IdentifierExpression { Identifier: var local }:
                return IsPidLocal(local, scope);
            default:
                return false;
        }
    }

    public bool HasOwnPid(Expression value, AstNode scope)
    {
        while (value is CastExpression or ParenthesizedExpression)
        {
            value = value is CastExpression cast
                ? cast.Expression
                : ((ParenthesizedExpression)value).Expression;
        }
        switch (value)
        {
            case InvocationExpression
            {
                Target: IdentifierExpression
                    or MemberReferenceExpression
                    {
                        Target: ThisReferenceExpression or BaseReferenceExpression
                    }
            } call:
                return call.GetResolveResult() is InvocationResolveResult { Member: IMethod method }
                    && IsOwnPid(method);
            case IdentifierExpression
            or MemberReferenceExpression
            {
                Target: ThisReferenceExpression or BaseReferenceExpression
            } when value.GetResolveResult() is MemberResolveResult { Member: IField field }:
                return GetOwnFields().Contains(field.MemberDefinition);
            case IdentifierExpression { Identifier: var local }
                when value.GetResolveResult()
                    is ILVariableResolveResult { Variable.Kind: not VariableKind.Parameter }:
                var sources = FindSources(local, scope);
                return sources.Count > 0 && sources.All(source => HasOwnPid(source, scope));
            default:
                return false;
        }
    }

    private static bool IsOwnPid(IMethod method)
    {
        return method.Name == OwnPid
            && method.Parameters.Count == 0
            && method.DeclaringTypeDefinition is ITypeDefinition type
            && type.GetAllBaseTypeDefinitions().Any(IsWindow);
    }

    private static bool IsWindow(ITypeDefinition type) =>
        type.Name == WindowType && type.Namespace.Length == 0;

    private bool ReturnsPid(IMethod method)
    {
        if (IsOwnPid(method))
        {
            return true;
        }
        if (!GameAssembly.IsGameDeclared(method) || method.Parameters.Count > 0 || !method.HasBody)
        {
            return false;
        }
        return Memoize(
            (IMethod)method.MemberDefinition,
            () =>
            {
                SyntaxTree tree = game.Decompile(method);
                AstNode scope = tree.Descendants.OfType<EntityDeclaration>().First();
                var returns = tree
                    .Descendants.OfType<ReturnStatement>()
                    .Select(statement => statement.Expression)
                    .ToList();
                return returns.Count > 0 && returns.All(value => HasPid(value, scope));
            }
        );
    }

    private bool StoresPid(IField field)
    {
        if (GetOwnFields().Contains(field.MemberDefinition))
        {
            return true;
        }
        if (
            field.ParentModule?.IsMainModule != true
            || field.DeclaringTypeDefinition is not ITypeDefinition type
        )
        {
            return false;
        }
        return Memoize(
            field.MemberDefinition,
            () =>
            {
                var sources = new List<(Expression Value, AstNode Scope)>();
                foreach (AstNode node in GetTree(type).Descendants)
                {
                    switch (node)
                    {
                        case AssignmentExpression
                        {
                            Operator: AssignmentOperatorType.Assign
                        } assignment when IsField(assignment.Left, field):
                            sources.Add((assignment.Right, GetScope(assignment)));
                            break;
                        case VariableInitializer
                        {
                            Parent: FieldDeclaration,
                            Initializer.IsNull: false
                        } initializer
                            when initializer.GetSymbol() is IField initializerField
                                && initializerField.MemberDefinition.Equals(field.MemberDefinition):
                            sources.Add((initializer.Initializer, GetScope(initializer)));
                            break;
                    }
                }
                return sources.Count > 0
                    && sources.All(source => HasPid(source.Value, source.Scope));
            }
        );
    }

    private bool IsPidPassed(string parameter, AstNode scope)
    {
        if (
            scope.GetSymbol() is not IMethod method
            || !GameAssembly.IsGameDeclared(method)
            || method.IsVirtual
            || method.IsOverride
            || method.IsAbstract
        )
        {
            return false;
        }
        IMethod definition = (IMethod)method.MemberDefinition;
        int index = definition.Parameters.ToList().FindIndex(own => own.Name == parameter);
        if (index < 0)
        {
            return false;
        }
        var key = ((ISymbol)definition, index);
        if (parameters.TryGetValue(key, out bool known))
        {
            return known;
        }
        parameters[key] = false;
        var values = new List<(Expression Value, AstNode Scope)>();
        foreach (
            ITypeDefinition type in game.FindCallerTypes(
                [definition],
                GameAssembly.Call,
                GameAssembly.CallVirtual,
                GameAssembly.NewObject,
                GameAssembly.LoadFunction,
                GameAssembly.LoadVirtualFunction
            )
        )
        {
            foreach (AstNode node in GetTree(type).Descendants)
            {
                if (IsDelegate(node, definition))
                {
                    return parameters[key] = false;
                }
                IReadOnlyList<Expression>? callArguments = node switch
                {
                    InvocationExpression call
                        when call.GetResolveResult()
                            is InvocationResolveResult { Member: IMethod target }
                            && target.MemberDefinition.Equals(definition) =>
                        call.Arguments.ToList(),
                    ObjectCreateExpression creation
                        when creation.GetResolveResult()
                            is InvocationResolveResult { Member: IMethod target }
                            && target.MemberDefinition.Equals(definition) =>
                        creation.Arguments.ToList(),
                    _ => null,
                };
                if (callArguments == null)
                {
                    continue;
                }
                if (
                    index >= callArguments.Count
                    || callArguments.Any(argument => argument is NamedArgumentExpression)
                )
                {
                    return parameters[key] = false;
                }
                values.Add((callArguments[index], GetScope(node)));
            }
        }
        return parameters[key] =
            values.Count > 0 && values.All(value => HasPid(value.Value, value.Scope));
    }

    private static bool IsDelegate(AstNode node, IMethod method)
    {
        if (
            node is not (IdentifierExpression or MemberReferenceExpression)
            || node.Parent is InvocationExpression call && call.Target == node
        )
        {
            return false;
        }
        return node.GetResolveResult() switch
        {
            MethodGroupResolveResult group => group.Methods.Any(candidate =>
                candidate.MemberDefinition.Equals(method)
            ),
            ConversionResolveResult { Conversion.Method: IMethod target } =>
                target.MemberDefinition.Equals(method),
            _ => false,
        };
    }

    private bool IsPidLocal(string local, AstNode scope)
    {
        var sources = FindSources(local, scope);
        return sources.Count > 0 && sources.All(source => HasPid(source, scope));
    }

    private static List<Expression> FindSources(string local, AstNode scope)
    {
        var sources = new List<Expression>();
        foreach (AstNode node in scope.Descendants)
        {
            switch (node)
            {
                case VariableInitializer { Initializer.IsNull: false } initializer
                    when initializer.Name == local:
                    sources.Add(initializer.Initializer);
                    break;
                case AssignmentExpression { Left: IdentifierExpression identifier } assignment
                    when identifier.Identifier == local:
                    sources.Add(assignment.Right);
                    break;
            }
        }
        return sources;
    }

    private HashSet<ISymbol> GetOwnFields()
    {
        if (ownFields != null)
        {
            return ownFields;
        }
        ownFields = [];
        ITypeDefinition window = game.Module.TypeDefinitions.Single(IsWindow);
        foreach (IMethod method in window.Methods.Where(IsOwnPid))
        {
            foreach (
                ReturnStatement statement in game.Decompile(method)
                    .Descendants.OfType<ReturnStatement>()
            )
            {
                if (
                    statement.Expression.GetResolveResult() is MemberResolveResult
                    {
                        Member: IField field
                    }
                )
                {
                    ownFields.Add(field.MemberDefinition);
                }
            }
        }
        return ownFields;
    }

    private static bool IsField(Expression target, IField field) =>
        target.GetResolveResult() is MemberResolveResult { Member: IField member }
        && member.MemberDefinition.Equals(field.MemberDefinition);

    private static AstNode GetScope(AstNode node) =>
        node.Ancestors.OfType<EntityDeclaration>().FirstOrDefault() ?? node;

    private SyntaxTree GetTree(ITypeDefinition type)
    {
        type = GameAssembly.GetOutermost(type);
        if (!types.TryGetValue(type, out SyntaxTree? tree))
        {
            types[type] = tree = game.DecompileType(type);
        }
        return tree;
    }

    private bool Memoize(ISymbol member, Func<bool> read)
    {
        if (members.TryGetValue(member, out bool known))
        {
            return known;
        }
        members[member] = false;
        return members[member] = read();
    }
}
