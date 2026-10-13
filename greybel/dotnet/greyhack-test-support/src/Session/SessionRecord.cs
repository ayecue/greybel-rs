using System.Text.Json;
using GreyHackTestSupport.Game;

namespace GreyHackTestSupport.Session;

/// <summary>
/// Shared-session file data for the game host process and its <see cref="GameConnection"/>, serialized as
/// <c>{"pid":N,"port":N,"testPort":N,"log":"…"}</c>.
/// </summary>
internal sealed record SessionRecord(int Pid, int Port, int TestPort, string Log)
{
    public static SessionRecord Create(int pid, GameConnection connection) =>
        new(pid, connection.Port, connection.TestPort, connection.Log);

    /// <summary>Parses a record from <paramref name="json"/>, returning null when no record is present.</summary>
    public static SessionRecord? Parse(string json)
    {
        try
        {
            return JsonSerializer.Deserialize<SessionRecord>(json, GameConnection.JsonOptions);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    public string ToJson() => JsonSerializer.Serialize(this, GameConnection.JsonOptions);
}
