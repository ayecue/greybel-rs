using System.Text.Json;

namespace GreyHackTestSupport.Game;

/// <summary>
/// Connection details available after game startup: message hook port, test-only protocol port, and game log path.
/// Serialized as one JSON line: <c>{"port":N,"testPort":N,"log":"…"}</c>.
/// </summary>
internal sealed record GameConnection(int Port, int TestPort, string Log)
{
    /// <summary>Uses camelCase for compatibility with test JSON readers.</summary>
    public static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    public string ToJson() => JsonSerializer.Serialize(this, JsonOptions);
}
