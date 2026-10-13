using GreyHackUtil.Contracts;

namespace GreyHackUtil.Tests.Support;

public static class ContractFixtures
{
    public static ContractFile CreateGame(params MessageContract[] server) =>
        new()
        {
            GameVersion = "1.0",
            FieldTypes =
            [
                new FieldType
                {
                    Key = "System.String",
                    Reader = "GetString",
                    ClrType = "System.String",
                    Type = ValueKind.String,
                },
                new FieldType
                {
                    Key = "System.Int32",
                    Reader = "GetInt",
                    ClrType = "System.Int32",
                    Type = ValueKind.Int32,
                },
                new FieldType
                {
                    Key = "System.Byte[]",
                    Reader = "GetByte",
                    ClrType = "System.Byte[]",
                    Type = ValueKind.Bytes,
                },
            ],
            Server = [.. server],
        };

    public static MessageContract CreateMessage(
        string name,
        int id,
        params (string Name, string FieldType)[] fields
    ) =>
        new()
        {
            Name = name,
            Id = id,
            Fields =
            [
                .. fields.Select(f => new FieldContract { Name = f.Name, FieldType = f.FieldType }),
            ],
        };

    public static ContractFile CreateClient(MessageContract client) =>
        new()
        {
            GameVersion = "1.0",
            FieldTypes = CreateGame().FieldTypes,
            Client = [client],
        };

    public static ContractSet CreateTypedCat() =>
        new(
            CreateGame(
                CreateMessage(
                    "CatServerRpc",
                    124,
                    ("zipContent", "System.Byte[]"),
                    ("terminalPID", "System.Int32")
                )
            ),
            new ContractFile()
        );

    public static MessageContract CreateWindowedPrint()
    {
        MessageContract print = CreateMessage(
            "PrintSentClientRpc",
            80,
            ("zipOutput", "System.Byte[]"),
            ("windowPID", "System.Int32")
        );
        print.Window = "windowPID";
        return print;
    }

    public static ContractSet CreateWindowedPrintSet() =>
        new(CreateClient(CreateWindowedPrint()), new ContractFile());
}
