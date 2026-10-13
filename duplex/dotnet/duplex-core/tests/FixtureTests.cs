using System.Text.Json.Nodes;
using Duplex.Tests.Calculator;
using Duplex.Tests.Layouts;
using Duplex.Tests.Support;
using static Duplex.Tests.Support.FixtureSetup;

namespace Duplex.Tests;

public class FixtureTests
{
    [Theory]
    [MemberData(nameof(GetLayoutNames), MemberType = typeof(FixtureSetup))]
    public void Should_lay_out_as_the_other_cores_do(string name)
    {
        var layout = LayoutCases[name];

        var actual = EncodeToHex(layout.Write);

        Assert.Equal(Fixtures[name], actual);
    }

    [Theory]
    [MemberData(nameof(GetLayoutNames), MemberType = typeof(FixtureSetup))]
    public void Should_read_a_layout_back_as_it_was(string name)
    {
        var reader = new DuplexReader(TestFrames.Unhex(Fixtures[name]));

        var rewrite = LayoutCases[name].Read(reader);

        reader.Finish();
        Assert.Equal(Fixtures[name], EncodeToHex(rewrite));
    }

    [Theory]
    [MemberData(nameof(GetBodyNames), MemberType = typeof(FixtureSetup))]
    public void Should_lay_out_a_message_body_as_the_other_cores_do(string name)
    {
        var actual = EncodeToHex(Bodies[name].Write);

        Assert.Equal(Fixtures[name], actual);
    }

    [Fact]
    public void Should_write_for_minor_0_as_the_other_cores_do()
    {
        var actual = EncodeToHex(new DefaultLayout().Write, minor: 0);

        Assert.Equal(Fixtures[DefaultsOfMinor0], actual);
    }

    [Fact]
    public void Should_read_the_defaults_of_what_minor_0_left_out()
    {
        var reader = new DuplexReader(TestFrames.Unhex(Fixtures[DefaultsOfMinor0])) { Minor = 0 };

        var defaults = DefaultLayout.Read(reader);

        Assert.Equal((byte)7, defaults.Level);
    }

    [Fact]
    public void Should_read_every_optional_state_back()
    {
        var notes = new[] { "progressBody", "progressAbsentBody", "progressNullBody" }
            .Select(name =>
                Calculator.Progress.Read(new DuplexReader(TestFrames.Unhex(Fixtures[name]))).Note
            )
            .Select(note => note.HasValue ? note.Value ?? "null" : "absent");

        Assert.Equal(["almost", "absent", "null"], notes);
    }

    [Fact]
    public void Should_refuse_a_text_that_is_not_utf8()
    {
        var reader = new DuplexReader(TestFrames.Unhex(Fixtures[MalformedTextBody]));

        var read = () => Log.Read(reader);

        Assert.Throws<MalformedException>(read);
    }

    [Fact]
    public void Should_read_the_frames_of_a_batch_of_the_other_cores()
    {
        var batch = TestFrames.Decode(TestFrames.Unhex(Fixtures[NotificationBatch]));

        Assert.Equal(
            new TestBatch(
                Calculation.Definition.ChannelId,
                Role.Client,
                1,
                [$"Message(type 0, id none, body {Fixtures["logBody"]})"]
            ).ToString(),
            batch.ToString()
        );
    }

    [Fact]
    public void Should_read_the_alias_a_batch_of_the_other_cores_announced()
    {
        var logTexts = new List<string>();
        var hub = new Hub(() => 0);
        var transport = new Transport(hub, new MemoryDriver());
        _ = new CalculatorChannel(
            transport,
            new Logger(logTexts),
            new ChannelOptions { WithoutHealthCheck = true }
        );
        hub.RunReady();
        transport.Receive(
            TestFrames.Encode(
                Calculation.Definition.ChannelId,
                Role.Client,
                0,
                (_, encoder) =>
                    encoder.WriteControl(
                        new Control(FrameKind.Open)
                        {
                            Core = new VersionNumber(1, 0),
                            Protocol = Calculation.Definition.Version,
                            Fingerprint = Calculation
                                .Definition.GetVersion(Calculation.Definition.Version.Minor)!
                                .Fingerprint,
                        }
                    )
            )
        );

        transport.Receive(TestFrames.Unhex(Fixtures[NotificationBatch]));
        transport.Receive(TestFrames.Unhex(Fixtures[AliasedBatch]));

        Assert.Equal(["adding", "adding"], logTexts);
    }

    [Fact]
    public void Should_cover_every_conformance_fixture()
    {
        var caseNames = LayoutCases
            .Keys.Concat(Bodies.Keys)
            .Concat([NotificationBatch, AliasedBatch, DefaultsOfMinor0, MalformedTextBody])
            .Concat(Omissions);

        Assert.Equal(
            Fixtures.Keys.Order(StringComparer.Ordinal),
            caseNames.Order(StringComparer.Ordinal)
        );
    }
}
