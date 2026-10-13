using System.Reflection;
using System.Text.Json.Nodes;
using Duplex.Tests.Calculator;

namespace Duplex.Tests.Support;

internal sealed class Scenario
{
    public static readonly string[] SupportedRequirements = ["health-check", "fragments"];

    private const uint Connection = 1;

    private readonly MemoryDriver driver = new();
    private readonly Hub hub;
    private readonly Transport transport;
    private readonly CalculatorChannel channel;
    private readonly JsonArray receivedMessages = [];
    private readonly JsonArray deprecations = [];
    private readonly TestFrames.Reader reader = new();
    private long now;

    private Scenario(JsonObject file)
    {
        hub = new Hub(() => now);
        transport = new Transport(hub, driver);
        var unhandledNames =
            file["unhandled"]?.AsArray().Select(name => (string)name!).ToHashSet() ?? [];
        var options = new ChannelOptions();
        if (file["healthCheck"]?.GetValue<bool>() == true)
        {
            options.HealthCheck = HealthCheck.Default;
        }
        else
        {
            options.WithoutHealthCheck = true;
        }
        if (file["unvalidated"]?.GetValue<bool>() == true)
        {
            options.Unvalidated = true;
        }
        channel = new CalculatorChannel(
            transport,
            new Handlers(unhandledNames, receivedMessages),
            options
        );
        channel.OnAck += connection =>
        {
            if (connection.DeprecatedVersion is { } version)
            {
                deprecations.Add(
                    new JsonObject
                    {
                        ["version"] = version.ToString(),
                        ["note"] = connection.DeprecationNote,
                    }
                );
            }
        };
    }

    public static string Duplex =>
        typeof(Scenario)
            .Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .Single(attribute => attribute.Key == "Duplex")
            .Value!;

    public static string Folder => Path.Combine(Duplex, "scenarios");

    public static IEnumerable<string> GetApplicable() =>
        Directory
            .GetFiles(Folder, "*.json")
            .Order(StringComparer.Ordinal)
            .Where(path => IsApplicable(Load(path)))
            .Select(path => Path.GetFileNameWithoutExtension(path));

    public static JsonObject Load(string path) =>
        JsonNode.Parse(File.ReadAllText(path))!.AsObject();

    public static bool IsApplicable(JsonObject file) =>
        (string?)file["peer"] == "provider"
        && ((string?)file["protocol"] ?? "calculator") == "calculator"
        && (file["major"]?.GetValue<int>() ?? 1) == 1
        && file["requires"]!
            .AsArray()
            .All(requirement => SupportedRequirements.Contains((string)requirement!));

    public static void Run(string name)
    {
        var file = Load(Path.Combine(Folder, name + ".json"));
        var scenario = new Scenario(file);
        scenario.hub.RunReady();
        if (file["connected"]?.GetValue<bool>() == true)
        {
            scenario.Connect();
        }
        var steps = file["steps"]!.AsArray();
        for (var index = 0; index < steps.Count; index++)
        {
            var step = steps[index]!.AsObject();
            try
            {
                scenario.Step(step);
            }
            catch (Exception error)
            {
                throw new Exception(
                    $"step {index}, {step.ToJsonString()}:\n{error.Message}",
                    error
                );
            }
        }
    }

    private void Connect()
    {
        Take();
        Receive(
            JsonNode.Parse("""{"connection": 0, "frames": [{"Open": {"id": 0}}]}""")!.AsObject()
        );
        hub.RunReady();
        Take();
        if (channel.Connection == null)
        {
            throw new Exception("didn't connect");
        }
    }

    private List<TestBatch> Take()
    {
        var batches = driver.SentBatches.SelectMany(reader.Read).ToList();
        driver.SentBatches.Clear();
        return batches;
    }

    private void Step(JsonObject step)
    {
        if (step["expect"] is JsonObject expectation)
        {
            Expect(expectation);
        }
        else if (step["receive"] is JsonObject batch)
        {
            Receive(batch);
        }
        else if (step["send"] is JsonObject message)
        {
            Notify(message, step["fails"]);
        }
        else if (step["check"] is JsonNode checks)
        {
            for (var check = 0; check < checks.GetValue<int>(); check++)
            {
                Advance(HealthCheck.Default.Interval);
            }
        }
        else if (step["wait"] is JsonNode wait)
        {
            Advance(wait.GetValue<long>());
        }
        else if (step["close"] is JsonNode reason)
        {
            channel.Close(Enum.Parse<ErrorCode>((string)reason!));
        }
        else if (step.ContainsKey("disconnect"))
        {
            transport.Disconnect();
        }
        else
        {
            throw new NotSupportedException("no step the C# runner takes");
        }
    }

    private void Advance(long milliseconds)
    {
        var end = now + milliseconds;
        while (now < end)
        {
            now++;
            hub.RunReady();
        }
    }

    private void Receive(JsonObject batch) =>
        transport.Receive(ScenarioBatches.Encode(batch, Role.Client));

    private void Notify(JsonObject message, JsonNode? fails)
    {
        JsonNode? outcome = null;
        try
        {
            var (name, fields) = ScenarioBatches.GetEntry(message);
            if (name != "Progress")
            {
                throw new NotSupportedException($"the provider sends no {name}");
            }
            channel.Send(ScenarioBatches.CreateProgress(fields));
        }
        catch (ChannelException error)
        {
            outcome = ToJson(error);
        }
        if (!JsonNode.DeepEquals(outcome, fails))
        {
            throw new Exception(
                $"notified: {outcome?.ToJsonString() ?? "null"}, expected {fails?.ToJsonString() ?? "null"}"
            );
        }
    }

    private static JsonNode ToJson(ChannelException error)
    {
        if (error.Violation is { } violation)
        {
            return new JsonObject
            {
                ["invalid"] = new JsonObject
                {
                    ["path"] = new JsonArray(
                        violation.Path.Select(position => (JsonNode)position).ToArray()
                    ),
                    ["requirement"] = violation.Requirement.ToString(),
                },
            };
        }
        return error.Kind switch
        {
            ChannelExceptionKind.Unavailable => "unsupported",
            ChannelExceptionKind.ChannelEnd => new JsonObject
            {
                ["closed"] = error.Code.ToString(),
            },
            _ => new JsonObject { ["rejected"] = error.Code.ToString() },
        };
    }

    private void Expect(JsonObject expectation)
    {
        hub.RunReady();
        var failures = new List<string>();
        void Compare(string label, JsonNode? actual, JsonNode? expected)
        {
            if (!JsonNode.DeepEquals(actual, expected))
            {
                failures.Add(
                    $"{label} {actual?.ToJsonString() ?? "null"}, expected {expected?.ToJsonString() ?? "null"}"
                );
            }
        }
        if (expectation["sent"] is JsonArray sentBatches)
        {
            var expectedTexts = sentBatches.Select(batch =>
                TestFrames
                    .Decode(ScenarioBatches.Encode(batch!.AsObject(), Role.Provider))
                    .ToString()
            );
            var actual = Take().Select(batch => batch.ToString());
            Compare(
                "sent",
                new JsonArray(actual.Select(text => (JsonNode)text).ToArray()),
                new JsonArray(expectedTexts.Select(text => (JsonNode)text).ToArray())
            );
        }
        if (expectation["received"] is JsonArray expectedEntries)
        {
            var actual = receivedMessages.DeepClone();
            receivedMessages.Clear();
            Compare("received", actual, expectedEntries);
        }
        if (expectation["health"] is JsonNode health)
        {
            Compare("health", channel.Health.ToString().ToLowerInvariant(), health);
        }
        if (expectation["deprecated"] is JsonArray expectedDeprecations)
        {
            var actual = deprecations.DeepClone();
            deprecations.Clear();
            Compare("deprecated", actual, expectedDeprecations);
        }
        if (expectation.ContainsKey("version"))
        {
            Compare("version", channel.Version?.ToString(), expectation["version"]);
        }
        if (expectation.ContainsKey("connection"))
        {
            Compare(
                "connection",
                channel.Connection is { } connection ? (JsonNode)connection : null,
                expectation["connection"]
            );
        }
        if (expectation.ContainsKey("closed"))
        {
            Compare("closed", channel.CloseReason?.ToString(), expectation["closed"]);
        }
        foreach (var key in expectation.Select(entry => entry.Key))
        {
            if (
                key
                is not (
                    "sent"
                    or "received"
                    or "health"
                    or "deprecated"
                    or "version"
                    or "connection"
                    or "closed"
                )
            )
            {
                failures.Add($"the C# runner expects no {key}");
            }
        }
        if (failures.Count > 0)
        {
            throw new Exception(string.Join("\n", failures));
        }
    }

    private sealed class Handlers(HashSet<string> unhandledNames, JsonArray receivedMessages)
        : CalculatorRouter
    {
        public override void OnLog(Log message)
        {
            if (!unhandledNames.Contains("Log"))
            {
                receivedMessages.Add(
                    new JsonObject { ["Log"] = new JsonObject { ["text"] = message.Text } }
                );
            }
        }
    }
}

internal static class ScenarioBatches
{
    private const int CoreOffset = 0;

    public static (string Name, JsonNode? Fields) GetEntry(JsonObject value)
    {
        if (value.Count != 1)
        {
            throw new InvalidOperationException($"not one entry: {value.ToJsonString()}");
        }
        var entry = value.First();
        return (entry.Key, entry.Value);
    }

    private static (string Name, JsonNode? Fields) GetMessageEntry(JsonObject fields)
    {
        var entry = fields.First(entry => entry.Key != "id");
        return (entry.Key, entry.Value);
    }

    public static Progress CreateProgress(JsonNode? fields)
    {
        var phase = Enum.Parse<Phase>((string)fields!["phase"]!);
        var eta = fields["eta"]?.GetValue<double>();
        return new Progress(phase, null, [], new Dictionary<string, uint>(), default, eta);
    }

    private static Message CreateMessage(string name, JsonNode? fields) =>
        name switch
        {
            "Log" => new Log((string)fields!["text"]!),
            "Add" => new Add(fields!["a"]!.GetValue<long>(), fields["b"]!.GetValue<long>()),
            "Divide" => new Divide(
                fields!["dividend"]!.GetValue<long>(),
                fields["divisor"]!.GetValue<long>(),
                fields["precision"]?.GetValue<byte>()
            ),
            "Reset" => new Reset(),
            "Count" => new Count(fields!["to"]!.GetValue<uint>()),
            "Sum" => new Sum(),
            "Confirm" => new Confirm((string)fields!["question"]!),
            "Progress" => CreateProgress(fields),
            _ => throw new NotSupportedException($"no message {name}"),
        };

    private static uint GetTypeId(string name) =>
        name switch
        {
            "Log" => Log.MessageType.Id,
            "Add" => Add.MessageType.Id,
            "Divide" => Divide.MessageType.Id,
            "Reset" => Reset.MessageType.Id,
            "Count" => Count.MessageType.Id,
            "Sum" => Sum.MessageType.Id,
            "Confirm" => Confirm.MessageType.Id,
            "Progress" => Duplex.Tests.Calculator.Progress.MessageType.Id,
            _ => throw new NotSupportedException($"no message {name}"),
        };

    private static VersionNumber ParseVersion(JsonNode? text, VersionNumber fallback)
    {
        if (text == null)
        {
            return fallback;
        }
        var parts = ((string)text!).Split('.');
        return new VersionNumber(ushort.Parse(parts[0]), ushort.Parse(parts[1]));
    }

    private static uint GetFingerprint(VersionNumber version)
    {
        var protocol = Calculation.Definition;
        return version.Major == protocol.Version.Major
            ? protocol.GetVersion(version.Minor)?.Fingerprint ?? 0
            : 0;
    }

    private static byte[] Encode(ushort minor, Action<DuplexWriter> write)
    {
        var writer = new DuplexWriter { Minor = minor };
        write(writer);
        return writer.ToArray();
    }

    public static byte[] Encode(JsonObject value, Role from)
    {
        if (value["bytes"] is JsonNode bytes)
        {
            return TestFrames.Unhex((string)bytes!);
        }
        var minor = value["minor"]?.GetValue<ushort>() ?? DuplexWriter.Newest;
        var channel = value["channel"]?.GetValue<uint>() ?? Calculation.Definition.ChannelId;
        var sender = value["from"] is JsonNode fromNode
            ? ((string)fromNode! == "client" ? Role.Client : Role.Provider)
            : from;
        var connection = value["connection"]!.GetValue<uint>();
        var batch = TestFrames.Encode(
            channel,
            sender,
            connection,
            (writer, encoder) =>
            {
                foreach (var frame in value["frames"]!.AsArray())
                {
                    WriteFrame(writer, encoder, frame!.AsObject(), minor);
                }
            }
        );
        if (value["core"] is JsonNode core)
        {
            batch[CoreOffset] = core.GetValue<byte>();
        }
        if (value["truncate"] is JsonNode truncate)
        {
            batch = batch[..^truncate.GetValue<int>()];
        }
        return batch;
    }

    private static byte[] EncodeFrame(JsonObject frame, ushort minor)
    {
        var writer = new DuplexWriter();
        var encoder = new FrameEncoder(writer);
        WriteFrame(writer, encoder, frame, minor);
        return writer.ToArray();
    }

    private static void WriteFrame(
        DuplexWriter writer,
        FrameEncoder encoder,
        JsonObject value,
        ushort minor
    )
    {
        var (kind, node) = GetEntry(value);
        var fields = node?.AsObject() ?? [];
        switch (kind)
        {
            case "Message":
            {
                uint? id = fields["id"]?.GetValue<uint>();
                if (fields["body"] is JsonNode body)
                {
                    encoder.WriteEncodedMessage(
                        GetTypeId((string)fields["message"]!),
                        id,
                        TestFrames.Unhex((string)body!)
                    );
                    break;
                }
                var (name, message) = GetMessageEntry(fields);
                var outgoing = CreateMessage(name, message);
                encoder.WriteEncodedMessage(
                    outgoing.GetMessageType().Id,
                    id,
                    Encode(minor, outgoing.Write)
                );
                break;
            }
            case "Failure":
            {
                var id = fields["id"]!.GetValue<uint>();
                if (fields["rejected"] is not JsonNode code)
                {
                    throw new NotSupportedException("the C# runner writes no declared failures");
                }
                Violation? violation = null;
                if (fields["violation"] is JsonObject violationObject)
                {
                    var path = violationObject["path"]!
                        .AsArray()
                        .Select(position => position!.GetValue<uint>())
                        .ToArray();
                    violation = new Violation(
                        path,
                        Enum.Parse<Requirement>((string)violationObject["requirement"]!)
                    );
                }
                encoder.WriteRejection(id, Enum.Parse<ErrorCode>((string)code!), violation);
                break;
            }
            case "Fragment":
            {
                var frame = EncodeFrame(fields["of"]!.AsObject(), minor);
                var start = fields["from"]?.GetValue<int>() ?? 0;
                var end = fields["to"]?.GetValue<int>() ?? frame.Length;
                var payload = new byte[end - start + 1];
                payload[0] = end == frame.Length ? FrameLayout.LastPiece : (byte)0;
                Array.Copy(frame, start, payload, 1, end - start);
                TestFrames.WriteRawFrame(writer, encoder, (uint)FrameKind.Fragment, payload);
                break;
            }
            case "Unknown":
                TestFrames.WriteRawFrame(
                    writer,
                    encoder,
                    fields["kind"]!.GetValue<uint>(),
                    TestFrames.Unhex((string)fields["payload"]!)
                );
                break;
            case "Listen":
            case "Ping":
            case "Pong":
                encoder.WriteControl(new Control(Enum.Parse<FrameKind>(kind)));
                break;
            case "Open":
            {
                var protocol = ParseVersion(fields["protocol"], Calculation.Definition.Version);
                encoder.WriteControl(
                    new Control(FrameKind.Open)
                    {
                        Id = fields["id"]!.GetValue<uint>(),
                        Core = ParseVersion(fields["core"], new VersionNumber(1, 0)),
                        Protocol = protocol,
                        Fingerprint =
                            fields["fingerprint"]?.GetValue<uint>() ?? GetFingerprint(protocol),
                        Payload = EncodePayload(fields),
                    }
                );
                break;
            }
            case "Ack":
            {
                var protocol = ParseVersion(fields["protocol"], Calculation.Definition.Version);
                encoder.WriteControl(
                    new Control(FrameKind.Ack)
                    {
                        Id = fields["id"]!.GetValue<uint>(),
                        Connection = fields["connection"]!.GetValue<uint>(),
                        Core = ParseVersion(fields["core"], new VersionNumber(1, 0)),
                        Protocol = protocol,
                        Fingerprint =
                            fields["fingerprint"]?.GetValue<uint>() ?? GetFingerprint(protocol),
                        Deprecation = (string?)fields["deprecation"],
                        Payload = EncodePayload(fields),
                    }
                );
                break;
            }
            case "Refuse":
                encoder.WriteControl(
                    new Control(FrameKind.Refuse)
                    {
                        Id = fields["id"]!.GetValue<uint>(),
                        Reason = Enum.Parse<ErrorCode>((string)fields["reason"]!),
                        Core = ParseVersion(fields["core"], new VersionNumber(1, 0)),
                        Protocol = ParseVersion(fields["protocol"], Calculation.Definition.Version),
                        Majors =
                            fields["majors"]
                                ?.AsArray()
                                .Select(major => major!.GetValue<ushort>())
                                .ToArray()
                            ?? [Calculation.Definition.Version.Major],
                        Payload = EncodePayload(fields),
                    }
                );
                break;
            case "Close":
                encoder.WriteControl(
                    new Control(FrameKind.Close)
                    {
                        Reason = Enum.Parse<ErrorCode>((string)fields["reason"]!),
                    }
                );
                break;
            default:
                throw new NotSupportedException($"the C# runner writes no {kind} frames");
        }
    }

    private static byte[] EncodePayload(JsonObject fields)
    {
        if (fields["payloadBytes"] is JsonNode bytes)
        {
            return TestFrames.Unhex((string)bytes!);
        }
        if (fields["payload"] != null)
        {
            throw new NotSupportedException("the C# core serves no handshakes");
        }
        return [];
    }
}
