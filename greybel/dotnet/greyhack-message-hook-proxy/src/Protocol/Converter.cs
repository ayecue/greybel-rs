#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Duplex;
using GreyHackMessageHook.Game;
using Newtonsoft.Json.Linq;

namespace GreyHackMessageHook.Protocol;

public sealed class Converter
{
    public const string GameVersionTag = "game_version";

    public const string GameMessageIdTag = "game_message_id";

    public const string PluginMessageIdTag = "plugin_message_id";

    public const string GameParamTypeTag = "game_param_type";

    public const string GameParamOptionalTag = "game_param_optional";

    public const string PidTag = "pid";

    public const string GameFieldNameTag = "game_field_name";

    private readonly Dictionary<Type, StructType> structs = new Dictionary<Type, StructType>();
    private readonly Dictionary<MessageType, Layout> byType = new Dictionary<MessageType, Layout>();
    private readonly Dictionary<(Role Sender, int GameId), Layout> byGameId =
        new Dictionary<(Role Sender, int GameId), Layout>();

    public Converter()
        : this(GreyHack.Definition) { }

    public Converter(ProtocolDefinition definition)
    {
        var problems = new List<string>();
        GameVersion = Tag.Find(definition.Tags, GameVersionTag)?.Value as string ?? "";
        if (GameVersion.Length == 0)
        {
            problems.Add($"the protocol has no {GameVersionTag}");
        }
        foreach (MessageType message in definition.Messages.SelectMany(messages => messages))
        {
            if (Layout.Create(message, this, problems) is Layout layout)
            {
                byType[message] = layout;
                var key = (message.Sender, layout.GameId);
                if (byGameId.ContainsKey(key))
                {
                    problems.Add(
                        $"{message.Name} has the game id {layout.GameId} of another message"
                    );
                }
                else
                {
                    byGameId[key] = layout;
                }
            }
        }
        if (problems.Count > 0)
        {
            throw new InvalidOperationException(
                "the protocol's messages don't convert to the game's: "
                    + string.Join("; ", problems)
            );
        }
    }

    public string GameVersion { get; }

    public RawMessage Encode(Message message)
    {
        if (!byType.TryGetValue(message.GetMessageType(), out Layout? layout))
        {
            throw new ArgumentException(
                $"{message.GetMessageType().Name} isn't a message of the protocol"
            );
        }
        var raw = new RawMessage(layout.GameId);
        for (int index = 0; index < layout.Message.Fields.Count; index++)
        {
            FieldType field = layout.Message.Fields[index];
            object? value = field.Get(message);
            if (
                value == null
                && layout.OptionalParameters.Contains(index)
                && layout.Message.Fields.Skip(index).All(f => f.Get(message) == null)
            )
            {
                break;
            }
            raw.Add(Write(value, field.Type));
        }
        return raw;
    }

    public Message? Decode(RawMessage raw, Role sender)
    {
        if (!byGameId.TryGetValue((sender, raw.Id), out Layout? layout))
        {
            return null;
        }
        RawMessageReader reader = raw.GetReader();
        object?[] values = layout
            .Message.Fields.Select(
                (field, index) =>
                    !reader.HasNext && layout.OptionalParameters.Contains(index)
                        ? null
                        : Read(reader.Next(), field.Type, AllowsNull(field))
            )
            .ToArray();
        return layout.Message.Create(values);
    }

    public bool IsExtension(RawMessage raw, Role sender)
    {
        return byGameId.TryGetValue((sender, raw.Id), out Layout? layout) && layout.IsExtension;
    }

    public bool IsExtension(Message message)
    {
        return byType.TryGetValue(message.GetMessageType(), out Layout? layout)
            && layout.IsExtension;
    }

    public WindowPidContainer GetWindowPids(Message message)
    {
        if (
            !byType.TryGetValue(message.GetMessageType(), out Layout? layout)
            || layout.PidField == null
        )
        {
            return Protocol.WindowPidContainer.None;
        }
        return layout.PidField.Get(message) switch
        {
            int pid => Protocol.WindowPidContainer.FromPid(pid),
            IReadOnlyList<int> pids => Protocol.WindowPidContainer.FromPids(pids.ToArray()),
            byte[] payload => Protocol.WindowPidContainer.FromPayload(payload),
            _ => Protocol.WindowPidContainer.None,
        };
    }

    private object? Write(object? value, Type type)
    {
        if (value == null || !structs.TryGetValue(type, out StructType? shape))
        {
            return value;
        }
        var json = new JObject();
        foreach (FieldType field in shape.Fields)
        {
            json[GetGameName(field)] = JToken.FromObject(field.Get(value)!);
        }
        return json;
    }

    private static bool AllowsNull(FieldType field)
    {
        return Tag.Find(field.Tags, GameParamOptionalTag) != null
            || Tag.Find(field.Tags, GameParamTypeTag)?.Value is string type
                && type.Contains("Nullable", StringComparison.Ordinal);
    }

    private object? Read(JToken value, Type type, bool allowsNull)
    {
        if (value.Type == JTokenType.Null)
        {
            return allowsNull ? null : CreateEmpty(type);
        }
        if (structs.TryGetValue(type, out StructType? shape))
        {
            return shape.Create(
                shape
                    .Fields.Select(field => value[GetGameName(field)]!.ToObject(field.Type))
                    .ToArray()
            );
        }
        return value.ToObject(type);
    }

    private static string GetGameName(FieldType field) =>
        (string)Tag.Find(field.Tags, GameFieldNameTag)!.Value!;

    private void CheckStruct(Type type, string where, List<string> problems)
    {
        if (structs.ContainsKey(type) || !type.IsClass || type == typeof(string) || type.IsArray)
        {
            return;
        }
        if (
            !(
                type.GetField(nameof(StructType), BindingFlags.Public | BindingFlags.Static)
                    ?.GetValue(null)
                is StructType shape
            )
        )
        {
            return;
        }
        foreach (
            FieldType field in shape.Fields.Where(field =>
                Tag.Find(field.Tags, GameFieldNameTag) == null
            )
        )
        {
            problems.Add($"{where}: {shape.Name}.{field.Name} has no {GameFieldNameTag}");
        }
        structs[type] = shape;
    }

    private static object CreateEmpty(Type type)
    {
        if (type == typeof(string))
        {
            return "";
        }
        if (type.IsArray)
        {
            return Array.CreateInstance(type.GetElementType()!, 0);
        }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(IReadOnlyList<>))
        {
            return Array.CreateInstance(type.GetGenericArguments()[0], 0);
        }
        return Activator.CreateInstance(type)!;
    }

    private sealed class Layout
    {
        private Layout(
            MessageType message,
            int gameId,
            bool isExtension,
            HashSet<int> optionalParameters,
            FieldType? pidField
        )
        {
            Message = message;
            GameId = gameId;
            IsExtension = isExtension;
            OptionalParameters = optionalParameters;
            PidField = pidField;
        }

        public MessageType Message { get; }

        public int GameId { get; }

        public bool IsExtension { get; }

        public HashSet<int> OptionalParameters { get; }

        public FieldType? PidField { get; }

        public static Layout? Create(
            MessageType message,
            Converter converter,
            List<string> problems
        )
        {
            int count = problems.Count;
            Tag? game = Tag.Find(message.Tags, GameMessageIdTag);
            Tag? extension = Tag.Find(message.Tags, PluginMessageIdTag);
            if ((game == null) == (extension == null))
            {
                problems.Add(
                    $"{message.Name} has a {GameMessageIdTag} or an {PluginMessageIdTag}, one of them"
                );
            }
            var optionalParameters = new HashSet<int>();
            for (int index = 0; index < message.Fields.Count; index++)
            {
                FieldType field = message.Fields[index];
                if (Tag.Find(field.Tags, GameParamOptionalTag) != null)
                {
                    optionalParameters.Add(index);
                }
                if (game != null && Tag.Find(field.Tags, GameParamTypeTag) == null)
                {
                    problems.Add($"{message.Name}.{field.Name} has no {GameParamTypeTag}");
                }
                converter.CheckStruct(field.Type, $"{message.Name}.{field.Name}", problems);
            }
            if (
                optionalParameters.Count > 0
                && optionalParameters.Min() != message.Fields.Count - optionalParameters.Count
            )
            {
                problems.Add($"{message.Name} has a non-trailing optional game parameter");
            }
            var pidFields = message
                .Fields.Where(field => Tag.Find(field.Tags, PidTag) != null)
                .ToList();
            if (pidFields.Count > 1)
            {
                problems.Add(
                    $"{message.Name} has {pidFields.Count} fields tagged {PidTag}, not one"
                );
            }
            FieldType? pidField = pidFields.FirstOrDefault();
            if (
                pidField != null
                && !(
                    pidField.Type == typeof(int)
                    || pidField.Type == typeof(IReadOnlyList<int>)
                    || pidField.Type == typeof(byte[])
                )
            )
            {
                problems.Add(
                    $"{message.Name}.{pidField.Name} is tagged {PidTag} but holds {pidField.Type}, not window pids"
                );
            }
            if (problems.Count > count)
            {
                return null;
            }
            Tag id = game ?? extension!;
            return new Layout(
                message,
                (int)(uint)id.Value!,
                extension != null,
                optionalParameters,
                pidField
            );
        }
    }
}
