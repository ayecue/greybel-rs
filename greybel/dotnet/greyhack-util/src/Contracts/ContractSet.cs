namespace GreyHackUtil.Contracts;

public enum Origin
{
    Game,

    Extension,
}

public sealed record Message(
    Origin Origin,
    string Name,
    int Id,
    string? Handler,
    Field? Window,
    IReadOnlyList<Field>? Fields
)
{
    public IReadOnlyList<string>? Senders { get; init; }

    public bool IsTyped => Fields != null;
}

public sealed record Field(string Name, FieldType FieldType, bool Optional = false);

public sealed record Direction(string Name, string IdEnum, IReadOnlyList<Message> Messages)
{
    public const string ClientName = "Client";
    public const string ClientIdEnum = "IdClient";
    public const string ServerName = "Server";
    public const string ServerIdEnum = "IdServer";

    public IEnumerable<Message> TypedMessages => Messages.Where(m => m.IsTyped);
}

public sealed class ContractSet
{
    public const string GameFile = "game.json";

    public const string GameBundle = "game.bundle";
    public const string ExtensionFile = "extension.json";

    public string GameVersion { get; }

    public IReadOnlyList<FieldType> FieldTypes { get; }

    public Direction Server { get; }

    public Direction Client { get; }

    public IEnumerable<Direction> Directions => [Server, Client];

    public ContractSet(ContractFile game, ContractFile extension)
    {
        GameVersion =
            game.GameVersion ?? throw new ContractException($"{GameFile} has no gameVersion");
        FieldTypes =
            game.FieldTypes ?? throw new ContractException($"{GameFile} has no fieldTypes");
        var fieldTypes = FieldTypes.ToDictionary(fieldType => fieldType.Key);
        Server = Merge(
            Direction.ServerName,
            Direction.ServerIdEnum,
            game.Server,
            extension.Server,
            fieldTypes
        );
        Client = Merge(
            Direction.ClientName,
            Direction.ClientIdEnum,
            game.Client,
            extension.Client,
            fieldTypes
        );
    }

    public static ContractSet Load(string directory)
    {
        return new ContractSet(
            LoadGame(directory),
            ContractJson.Load(Path.Combine(directory, ExtensionFile))
        );
    }

    public static ContractFile LoadGame(string directory)
    {
        string path = Path.Combine(directory, GameBundle);
        try
        {
            var (_, contents) = AssetBundle
                .BundleFile.Read(File.ReadAllBytes(path))
                .Single(file => file.Name == GameFile);
            return ContractJson.Parse(System.Text.Encoding.UTF8.GetString(contents), path);
        }
        catch (Exception error)
            when (error is IOException or InvalidDataException or InvalidOperationException)
        {
            throw new ContractException($"{path}: {error.Message}");
        }
    }

    public static void SaveGame(ContractFile game, string directory)
    {
        byte[] contents = System.Text.Encoding.UTF8.GetBytes(ContractJson.Serialize(game));
        AssetBundle.Fingerprint.WriteBundle(
            directory,
            GameBundle,
            game.GameVersion ?? throw new ContractException($"{GameFile} has no gameVersion"),
            [(GameFile, contents)]
        );
    }

    private static Direction Merge(
        string direction,
        string idEnum,
        List<MessageContract> game,
        List<MessageContract> extension,
        Dictionary<string, FieldType> fieldTypes
    )
    {
        var messages = game.Select(m => Resolve(Origin.Game, m, fieldTypes))
            .Concat(extension.Select(m => Resolve(Origin.Extension, m, fieldTypes)))
            .ToList();
        foreach (var group in messages.GroupBy(m => m.Id).Where(g => g.Count() > 1))
        {
            throw new ContractException(
                $"{direction}: id {group.Key} is used by {string.Join(" and ", group.Select(m => $"{m.Name} ({m.Origin})"))}"
            );
        }
        foreach (var group in messages.GroupBy(m => m.Name).Where(g => g.Count() > 1))
        {
            throw new ContractException($"{direction}: name {group.Key} is used twice");
        }
        return new Direction(direction, idEnum, messages);
    }

    private static Message Resolve(
        Origin origin,
        MessageContract message,
        Dictionary<string, FieldType> fieldTypes
    )
    {
        var fields = message
            .Fields?.Select(f => new Field(
                f.Name,
                fieldTypes.TryGetValue(f.FieldType, out FieldType? fieldType)
                    ? fieldType
                    : throw new ContractException(
                        $"{message.Name}.{f.Name}: unknown field type {f.FieldType}"
                    ),
                f.Optional
            ))
            .ToList();
        Field? window = null;
        if (message.Window != null)
        {
            window =
                fields?.FirstOrDefault(f => f.Name == message.Window)
                ?? throw new ContractException(
                    $"{message.Name}: window field {message.Window} is not a field"
                );
            if (
                window.FieldType.Type
                is not (ValueKind.Int32 or ValueKind.Int32Array or ValueKind.Bytes)
            )
            {
                throw new ContractException(
                    $"{message.Name}: window field {message.Window} holds {window.FieldType.Type}, not window pids"
                );
            }
        }
        return new Message(origin, message.Name, message.Id, message.Handler, window, fields)
        {
            Senders = message.Senders,
        };
    }
}
