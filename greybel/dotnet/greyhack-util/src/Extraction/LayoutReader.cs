using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Resolver;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.Semantics;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

internal sealed class LayoutReader(ValueTypeCatalog types, ICollection<string> warnings)
{
    public (IMethod? Handler, List<FieldContract> Fields) Read(
        AstNode scope,
        IEnumerable<Statement> statements,
        string where
    )
    {
        IMethod? handler = null;
        var reads = new List<(InvocationExpression Invocation, FieldType Type)>();
        foreach (
            InvocationExpression invocation in statements
                .SelectMany(s => s.DescendantsAndSelf)
                .OfType<InvocationExpression>()
        )
        {
            if (
                invocation.GetResolveResult()
                is not InvocationResolveResult { Member: IMethod method }
            )
            {
                continue;
            }
            if (method.DeclaringType.FullName != FieldTypeReader.MessageType)
            {
                handler ??=
                    GameAssembly.IsGameDeclared(method) && !IsWrapper(invocation) ? method : null;
                continue;
            }
            if (method.Name != FieldTypeReader.ParameterReader || method.TypeArguments.Count != 1)
            {
                throw new ContractException(
                    $"{where}: {FieldTypeReader.MessageType}.{method.Name} is not a parameter reader"
                );
            }
            reads.Add((invocation, types.Read(method.TypeArguments[0])));
        }

        var fields = new List<FieldContract>();
        for (int index = 0; index < reads.Count; index++)
        {
            var (invocation, fieldType) = reads[index];
            bool conditional = IsConditional(invocation, scope, reads.Select(r => r.Invocation));
            if (conditional && index != reads.Count - 1)
            {
                throw new ContractException(
                    $"{where}: {invocation} is read conditionally before a required parameter"
                );
            }
            string name = GetValueName(invocation, scope, where);
            string unique = name;
            for (int i = 2; fields.Any(f => f.Name == unique); i++)
            {
                unique = name + i;
            }
            fields.Add(
                new FieldContract
                {
                    Name = unique,
                    FieldType = fieldType.Key,
                    Optional = conditional,
                }
            );
        }
        return (handler, fields);
    }

    private static bool IsWrapper(InvocationExpression invocation) =>
        invocation.Arguments.Any(a => a is AnonymousMethodExpression or LambdaExpression);

    private static bool IsConditional(AstNode read, AstNode scope, IEnumerable<AstNode> reads)
    {
        AstNode child = read;
        for (
            AstNode? current = read.Parent;
            current != null && current != scope;
            child = current, current = current.Parent
        )
        {
            bool branches = current switch
            {
                IfElseStatement statement => child != statement.Condition,
                ConditionalExpression expression => child != expression.Condition,
                SwitchStatement statement => child != statement.Expression,
                BinaryOperatorExpression
                {
                    Operator: BinaryOperatorType.ConditionalAnd
                        or BinaryOperatorType.ConditionalOr
                        or BinaryOperatorType.NullCoalescing
                } expression => child == expression.Right,
                WhileStatement or ForStatement or ForeachStatement or DoWhileStatement => true,
                _ => false,
            };
            if (branches && !reads.All(r => r.Ancestors.Contains(current)))
            {
                return true;
            }
        }
        return false;
    }

    private string GetValueName(InvocationExpression read, AstNode scope, string where)
    {
        AstNode node = Unwrap(read);
        if (GetParameterName(node, where) is string parameter)
        {
            return parameter;
        }
        string local = node.Parent switch
        {
            VariableInitializer variable => variable.Name,
            AssignmentExpression { Left: IdentifierExpression identifier } => identifier.Identifier,
            _ => throw new ContractException($"{where}: cannot name the value read by {read}"),
        };
        foreach (
            IdentifierExpression use in scope
                .Descendants.OfType<IdentifierExpression>()
                .Where(i => i.Identifier == local)
        )
        {
            if (GetParameterName(Unwrap(use), where) is string name)
            {
                return name;
            }
        }
        return local;
    }

    private static AstNode Unwrap(AstNode node)
    {
        while (true)
        {
            if (
                node.Parent is CastExpression or ParenthesizedExpression or BinaryOperatorExpression
            )
            {
                node = node.Parent;
            }
            else if (
                node.Parent is MemberReferenceExpression member
                && member.Parent is InvocationExpression call
                && call.Target == member
            )
            {
                node = call;
            }
            else
            {
                return node;
            }
        }
    }

    private string? GetParameterName(AstNode argument, string where)
    {
        switch (argument.Parent)
        {
            case InvocationExpression call
                when call.GetResolveResult()
                    is InvocationResolveResult { Member: IMethod method } result:
            {
                int index = call.Arguments.TakeWhile(a => a != argument).Count();
                if (result is CSharpInvocationResolveResult { IsExtensionMethodInvocation: true })
                {
                    index++;
                }
                if (!GameAssembly.IsGameDeclared(method))
                {
                    warnings.Add(
                        $"{where}: value passed to {method.DeclaringType.FullName}.{method.Name}"
                    );
                }
                return index < method.Parameters.Count ? method.Parameters[index].Name : null;
            }
            case ObjectCreateExpression create
                when create.GetResolveResult() is InvocationResolveResult { Member: IMethod ctor }:
                return ctor.Parameters[create.Arguments.TakeWhile(a => a != argument).Count()].Name;
            default:
                return null;
        }
    }
}
