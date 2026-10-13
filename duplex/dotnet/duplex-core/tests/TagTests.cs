using Duplex.Tests.Sensor;
using static Duplex.Tests.Support.TagSetup;

namespace Duplex.Tests;

public class TagTests
{
    [Fact]
    public void ShouldKeepTheTagsOfTheProtocolAndItsMessages()
    {
        Assert.Equal([("origin", (object?)"lab")], ToPairs(Sensor.Sensor.Definition.Tags));
        Assert.Equal(
            [("origin", (object?)"probe"), ("number", 34u)],
            ToPairs(Reading.MessageType.Tags)
        );
        Assert.Equal([("number", (object?)1000u)], ToPairs(Calibrate.MessageType.Tags));
        Assert.Empty(Silence.MessageType.Tags);
    }

    [Fact]
    public void ShouldKeepTheTagsOfAMessagesFieldsInTheirOrder()
    {
        var fields = Reading
            .MessageType.Fields.Select(field => (field.Name, ToPairs(field.Tags).ToList()))
            .ToList();

        Assert.Equal(
            ["value", "station", "labels", "position", "shape", "unit", "note"],
            fields.Select(field => field.Name)
        );
        Assert.Equal(
            [("alias", (object?)"value"), ("alias", "measurement"), ("weight", 0.5)],
            fields[0].Item2
        );
        Assert.Equal([("recipient", (object?)null), ("offset", -3L)], fields[1].Item2);
        Assert.Equal([("sorted", (object?)true)], fields[2].Item2);
        Assert.Equal(
            "station",
            FieldType.FindWithTag(Reading.MessageType.Fields, "recipient")?.Name
        );
    }

    [Fact]
    public void ShouldKeepTheTagsOfTypesVariantsAndMembers()
    {
        Assert.Equal([("kind", (object?)Kind.Plain)], ToPairs(Position.StructType.Tags));
        Assert.Equal([("kind", (object?)Kind.Sum)], ToPairs(Shape.UnionType.Tags));
        Assert.Equal([("kind", (object?)Kind.Plain)], ToPairs(UnitCodec.EnumType.Tags));
        Assert.Empty(KindCodec.EnumType.Tags);
        Assert.Equal(
            [("alias", (object?)"celsius"), ("marker", (int)'C')],
            ToPairs(UnitCodec.EnumType.GetMember(1)!.Tags)
        );
        Assert.Equal(
            [("alias", (object?)"dot")],
            ToPairs(Shape.UnionType.GetVariant(new Shape.Point()).Tags)
        );
        Assert.Equal(
            [("recipient", (object?)null)],
            ToPairs(Shape.UnionType.GetVariant(new Shape.Box(1, 2)).Fields[0].Tags)
        );
    }

    [Fact]
    public void ShouldReadAndMakeAMessageByItsFields()
    {
        var reading = new Reading(
            1.5,
            3,
            ["north"],
            new Position(1, 2),
            new Shape.Box(4, 5),
            Unit.Kelvin
        );

        object?[] values = Reading.MessageType.Fields.Select(field => field.Get(reading)).ToArray();
        var copy = (Reading)Reading.MessageType.Create(values);

        Assert.Equal(
            new object?[]
            {
                1.5,
                3u,
                reading.Labels,
                reading.Position,
                reading.Shape,
                Unit.Kelvin,
                null,
            },
            values
        );
        Assert.Equal(
            (reading.Value, reading.Station, reading.Unit, reading.Note),
            (copy.Value, copy.Station, copy.Unit, copy.Note)
        );
        Assert.Same(reading.Position, copy.Position);
    }

    [Fact]
    public void ShouldMakeAStructByItsFields()
    {
        var position = (Position)Position.StructType.Create([3f, 4f]);

        Assert.Equal((3f, 4f), (position.Across, position.Down));
        Assert.Equal(3f, Position.StructType.Fields[0].Get(position));
    }
}
