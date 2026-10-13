using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.Semantics;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

public static class ContractExtractor
{
    private sealed record Receiver(string IdType, string ReceiverType);

    private static readonly Receiver Server = new("NetworkMessages.IdServer", "PlayerServer");
    private static readonly Receiver Client = new("NetworkMessages.IdClient", "PlayerClient");

    private const string ReceiveMethod = "ProcessReceivedData";
    private const string MessageServerType = "NetworkMessages.MessageServer";
    private const string MessageClientType = "NetworkMessages.MessageClient";

    private static readonly (
        Receiver Receiver,
        string Message,
        string ReceiverType,
        string Method
    )[] OtherReceivers = [(Server, "UserLoginServerRpc", "ServerListener", "IsValidClient")];

    public static ContractFile Extract(string assemblyPath, ICollection<string> warnings)
    {
        var game = new GameAssembly(assemblyPath);
        var fieldTypes = new ValueTypeCatalog();
        var layouts = new LayoutReader(fieldTypes, warnings);
        var windows = new WindowReader(game);
        var handlers = new Dictionary<MessageContract, IMethod>();
        var contract = new ContractFile
        {
            GameVersion = game.GameVersion,
            Server = ReadMessages(game, Server, layouts, windows, handlers),
            Client = ReadMessages(game, Client, layouts, windows, handlers),
        };
        var senders = new SenderReader(game, fieldTypes);
        var sentLayouts = senders.Read(
            new Dictionary<string, string>
            {
                [MessageServerType] = Server.IdType,
                [MessageClientType] = Client.IdType,
            },
            warnings
        );
        foreach (
            var (receiver, messages) in new[]
            {
                (Server, contract.Server),
                (Client, contract.Client),
            }
        )
        {
            foreach (MessageContract message in messages)
            {
                senders.Describe(
                    message,
                    sentLayouts.GetValueOrDefault((receiver.IdType, message.Name)) ?? [],
                    warnings
                );
            }
        }
        var replies = contract.Client.ToDictionary(message => message.Name);

        foreach (
            MessageContract message in contract.Server.Where(message =>
                message.Fields != null && message.Window == null
            )
        )
        {
            if (handlers.TryGetValue(message, out IMethod? handler))
            {
                message.Window = WindowReader.FromReply(
                    message,
                    handler,
                    sentLayouts,
                    Client.IdType,
                    replies,
                    warnings
                );
            }
        }
        contract.FieldTypes = fieldTypes.Types.ToList();
        return contract;
    }

    private static List<MessageContract> ReadMessages(
        GameAssembly game,
        Receiver receiver,
        LayoutReader layouts,
        WindowReader windows,
        Dictionary<MessageContract, IMethod> handlers
    )
    {
        var messages = game.FindType(receiver.IdType)
            .Fields.Where(f => f.IsConst)
            .Select(f => new MessageContract
            {
                Name = f.Name,
                Id = Convert.ToInt32(f.GetConstantValue()),
            })
            .OrderBy(m => m.Id)
            .ToList();
        var byName = messages.ToDictionary(m => m.Name);

        SyntaxTree receive = game.Decompile(game.FindMethod(receiver.ReceiverType, ReceiveMethod));
        foreach (SwitchSection section in receive.Descendants.OfType<SwitchSection>())
        {
            string[] names = section
                .CaseLabels.Select(fieldType => fieldType.Expression.GetResolveResult())
                .OfType<MemberResolveResult>()
                .Where(r => r.Member.DeclaringType.FullName == receiver.IdType)
                .Select(r => r.Member.Name)
                .ToArray();
            if (names.Length == 0 || section.Statements.All(s => s is BreakStatement))
            {
                continue;
            }
            string where = $"{receiver.ReceiverType}: case {string.Join(", ", names)}";
            var (handler, fields) = layouts.Read(section, section.Statements, where);
            foreach (string name in names)
            {
                Describe(byName[name], handler, fields, windows, handlers, where);
            }
        }

        foreach (
            var (_, message, type, method) in OtherReceivers.Where(r => r.Receiver == receiver)
        )
        {
            SyntaxTree tree = game.Decompile(game.FindMethod(type, method));
            IEnumerable<Statement> body = tree
                .Descendants.OfType<Statement>()
                .Where(s => s.Parent is BlockStatement { Parent: MethodDeclaration });
            var (handler, fields) = layouts.Read(tree, body, $"{type}.{method}");
            Describe(byName[message], handler, fields, windows, handlers, $"{type}.{method}");
        }
        return messages;
    }

    private static void Describe(
        MessageContract message,
        IMethod? handler,
        List<FieldContract> fields,
        WindowReader windows,
        Dictionary<MessageContract, IMethod> handlers,
        string where
    )
    {
        message.Handler = handler == null ? null : $"{handler.DeclaringType.Name}.{handler.Name}";
        message.Fields = fields;
        if (handler == null)
        {
            return;
        }
        handlers[message] = handler;
        string? window = windows.Read(handler, where);
        if (window != null && fields.All(f => f.Name != window))
        {
            throw new ContractException(
                $"{where}: {message.Handler} looks windows up by {window}, which is not a field"
            );
        }
        message.Window = window;
    }
}
