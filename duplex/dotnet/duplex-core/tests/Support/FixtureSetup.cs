using System.Text.Json.Nodes;
using Duplex.Tests.Calculator;
using Duplex.Tests.Layouts;
using Layout = Duplex.Tests.Layouts;

namespace Duplex.Tests.Support;

internal static class FixtureSetup
{
    internal const string NotificationBatch = "notificationBatch";
    internal const string AliasedBatch = "aliasedBatch";
    internal const string DefaultsOfMinor0 = "defaultsOfMinor0";
    internal const string MalformedTextBody = "malformedTextBody";

    internal static readonly string[] Omissions =
    [
        "failureBatch",
        "openBatch",
        "secureFirst",
        "secureSecond",
        "secureThird",
        "secureClientBatch",
        "secureProviderBatch",
    ];

    internal static readonly Dictionary<string, string> Fixtures = JsonNode
        .Parse(File.ReadAllText(Path.Combine(Scenario.Duplex, "conformance", "fixtures.json")))!
        .AsObject()
        .ToDictionary(entry => entry.Key, entry => (string)entry.Value!);

    internal sealed record Layout(
        Action<DuplexWriter> Write,
        Func<DuplexReader, Action<DuplexWriter>> Read
    );

    internal static Layout Create<T>(
        T value,
        Action<T, DuplexWriter> write,
        Func<DuplexReader, T> read
    ) =>
        new(
            writer => write(value, writer),
            reader =>
            {
                var copy = read(reader);
                return writer => write(copy, writer);
            }
        );

    internal static readonly Dictionary<string, Layout> LayoutCases = new()
    {
        ["defaults"] = Create(
            new DefaultLayout(),
            (value, writer) => value.Write(writer),
            DefaultLayout.Read
        ),
        ["example"] = Create(
            new Example("test", 300, true),
            (value, writer) => value.Write(writer),
            Example.Read
        ),
        ["numbers"] = Create(
            new NumberLayout(
                255,
                65_535,
                4_000_000_000,
                9_007_199_254_740_991,
                -128,
                -32_768,
                int.MinValue,
                -9_007_199_254_740_991,
                -1.25f,
                0.1
            ),
            (value, writer) => value.Write(writer),
            NumberLayout.Read
        ),
        ["flags"] = Create(
            new Flags(
                true,
                true,
                null,
                new Optional<bool?>(null),
                new Optional<string?>("t"),
                false,
                true,
                true
            ),
            (value, writer) => value.Write(writer),
            Flags.Read
        ),
        ["otherFlags"] = Create(
            new Flags(false, null, false, new Optional<bool?>(true), default, true, false, false),
            (value, writer) => value.Write(writer),
            Flags.Read
        ),
        ["characters"] = Create(
            new CharacterLayout('é', ['a', null, 0x1f600]),
            (value, writer) => value.Write(writer),
            CharacterLayout.Read
        ),
        ["printed"] = Create(
            new PrintLayout("a", true, false),
            (value, writer) => value.Write(writer),
            PrintLayout.Read
        ),
        ["point"] = Create<Shape>(
            new Shape.Point(),
            (value, writer) => value.Write(writer),
            Shape.Read
        ),
        ["circle"] = Create<Shape>(
            new Shape.Circle(0.5),
            (value, writer) => value.Write(writer),
            Shape.Read
        ),
        ["rectangle"] = Create<Shape>(
            new Shape.Rectangle(2, 3, true),
            (value, writer) => value.Write(writer),
            Shape.Read
        ),
        ["keywords"] = Create(
            new KeywordLayout(Keyword.Match, [Keyword.Type]),
            (value, writer) => value.Write(writer),
            KeywordLayout.Read
        ),
        ["raw"] = Create(
            new Raw(
                0xdeadbeef,
                (1ul << 40) + 5,
                [1, 2, 3],
                [
                    [],
                    [255],
                ]
            ),
            (value, writer) => value.Write(writer),
            Raw.Read
        ),
        ["signals"] = Create(
            new SignalLayout(Signal.Kill, [Signal.Hang, Signal.Kill]),
            (value, writer) => value.Write(writer),
            SignalLayout.Read
        ),
    };

    internal static Progress CreateProgress(Optional<string?> note) =>
        new(
            Phase.Active,
            0.75,
            [
                new Step("exact", new Quotient.Exact(2)),
                new Step("remainder", new Quotient.WithRemainder(3, 1)),
                new Step("waiting", null),
            ],
            new Dictionary<string, uint> { ["done"] = 2, ["total"] = 3 },
            note
        );

    internal static readonly Dictionary<string, Message> Bodies = new()
    {
        ["logBody"] = new Log("adding"),
        ["progressBody"] = CreateProgress(new Optional<string?>("almost")),
        ["progressAbsentBody"] = CreateProgress(default),
        ["progressNullBody"] = CreateProgress(new Optional<string?>(null)),
    };

    internal static string EncodeToHex(
        Action<DuplexWriter> write,
        ushort minor = DuplexWriter.Newest
    )
    {
        var writer = new DuplexWriter { Minor = minor };
        write(writer);
        return TestFrames.ToHex(writer.ToArray());
    }

    public static TheoryData<string> GetLayoutNames() => new(LayoutCases.Keys);

    public static TheoryData<string> GetBodyNames() => new(Bodies.Keys);

    internal sealed class Logger(List<string> logTexts) : CalculatorRouter
    {
        public override void OnLog(Log message) => logTexts.Add(message.Text);
    }
}
