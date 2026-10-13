using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Resolver;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.IL;
using ICSharpCode.Decompiler.Semantics;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

internal sealed class WindowReader(GameAssembly game)
{
    private static readonly (string Type, string Method)[] Lookups =
    [
        ("PlayerClientMethods", "GetVentana"),
    ];

    public string? Read(IMethod handler, string where)
    {
        SyntaxTree tree = game.Decompile(handler);
        var sources = new List<string>();
        foreach (
            InvocationExpression lookup in tree
                .Descendants.OfType<InvocationExpression>()
                .Where(IsLookup)
        )
        {
            foreach (string parameter in FindSources(lookup.Arguments.First(), tree, []))
            {
                if (!sources.Contains(parameter))
                {
                    sources.Add(parameter);
                }
            }
        }
        return sources.Count switch
        {
            0 => null,
            1 => sources[0],
            _ => throw new ContractException(
                $"{where}: {handler.Name} looks windows up by {string.Join(" and ", sources)}; the contract format has one window field"
            ),
        };
    }

    private static bool IsLookup(InvocationExpression invocation)
    {
        return invocation.GetResolveResult() is InvocationResolveResult { Member: IMethod method }
            && Lookups.Contains((method.DeclaringType.Name, method.Name));
    }

    private static IEnumerable<string> FindSources(
        AstNode node,
        SyntaxTree tree,
        HashSet<string> visitedNames
    )
    {
        if (
            node is IdentifierExpression identifier
            && identifier.GetResolveResult()
                is ILVariableResolveResult { Variable: { Name: string name } variable }
        )
        {
            if (variable.Kind == VariableKind.Parameter)
            {
                return [name];
            }
            if (!visitedNames.Add(name))
            {
                return [];
            }
            return FindAssignments(name, tree)
                .SelectMany(value => FindSources(value, tree, visitedNames))
                .ToList();
        }
        return node.Children.SelectMany(child => FindSources(child, tree, visitedNames)).ToList();
    }

    private static IEnumerable<AstNode> FindAssignments(string local, SyntaxTree tree)
    {
        foreach (AstNode node in tree.Descendants)
        {
            switch (node)
            {
                case VariableInitializer variable
                    when variable.Name == local && !variable.Initializer.IsNull:
                    yield return variable.Initializer;
                    break;
                case AssignmentExpression { Left: IdentifierExpression identifier } assignment
                    when identifier.Identifier == local:
                    yield return assignment.Right;
                    break;
                case ForeachStatement loop
                    when loop.VariableDesignation
                        is SingleVariableDesignation { Identifier: var name }
                        && name == local:
                    yield return loop.InExpression;
                    break;
            }
        }
    }

    /// <summary>
    /// Gets the <paramref name="message"/> field containing its window pid. Senders normally set this field to their
    /// own window pid; otherwise any window pid is accepted. Returns null and logs a warning if multiple fields match.
    /// </summary>
    internal static string? FromSenders(
        MessageContract message,
        IReadOnlyList<FieldContract> fields,
        IReadOnlyList<SentLayout> layouts,
        ICollection<string> warnings
    )
    {
        if (layouts.Count == 0)
        {
            return null;
        }
        foreach (
            Func<SentValue, bool> window in new Func<SentValue, bool>[]
            {
                value => value.OwnPid,
                value => value.WindowPid,
            }
        )
        {
            var windows = new List<string>();
            for (int index = 0; index < fields.Count; index++)
            {
                if (
                    layouts.All(layout =>
                        layout.Values.ElementAtOrDefault(index) is SentValue value && window(value)
                    )
                )
                {
                    windows.Add(fields[index].Name);
                }
            }
            if (windows.Count > 1)
            {
                warnings.Add(
                    $"{message.Name}: its senders send windows' pids as {string.Join(" and ", windows)}; the contract format has one window field"
                );
                return null;
            }
            if (windows.Count == 1)
            {
                return windows[0];
            }
        }
        return null;
    }

    /// <summary>
    /// Gets the <paramref name="message"/> field containing its window pid by inspecting replies sent by
    /// <paramref name="handler"/>. The matching field is forwarded as the reply window. Returns null and logs a warning
    /// if multiple fields match.
    /// </summary>
    public static string? FromReply(
        MessageContract message,
        IMethod handler,
        Dictionary<(string IdType, string Message), List<SentLayout>> layouts,
        string replyIdType,
        IReadOnlyDictionary<string, MessageContract> replies,
        ICollection<string> warnings
    )
    {
        var windows = new HashSet<string>();
        foreach (var ((idType, name), messageLayouts) in layouts)
        {
            if (
                idType != replyIdType
                || !replies.TryGetValue(name, out MessageContract? reply)
                || reply.Window == null
                || reply.Fields == null
            )
            {
                continue;
            }
            int index = reply.Fields.TakeWhile(field => field.Name != reply.Window).Count();
            foreach (
                SentLayout layout in messageLayouts.Where(layout =>
                    layout.Method?.MemberDefinition == handler.MemberDefinition
                )
            )
            {
                if (layout.Values.ElementAtOrDefault(index)?.Parameter is string parameter)
                {
                    windows.Add(parameter);
                }
            }
        }
        var fields = windows
            .Where(window => message.Fields!.Any(field => field.Name == window))
            .ToList();
        if (fields.Count > 1)
        {
            warnings.Add(
                $"{message.Name}: {handler.Name} answers to the windows of {string.Join(" and ", fields)}; the contract format has one window field"
            );
            return null;
        }
        return fields.SingleOrDefault();
    }
}
