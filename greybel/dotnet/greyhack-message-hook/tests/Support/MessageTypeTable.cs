using Duplex;
using GreyHackMessageHook.Protocol;

namespace GreyHackMessageHook.Tests.Support;

internal static class MessageTypeTable
{
    public static IEnumerable<MessageType> All =>
        GreyHack.Definition.Messages.SelectMany(messages => messages);

    public static MessageType GetMessageType(string name) => All.Single(type => type.Name == name);

    public static IEnumerable<object?> GetValues(MessageType type, Duplex.Message message) =>
        type.Fields.Select(field => field.Get(message));

}
