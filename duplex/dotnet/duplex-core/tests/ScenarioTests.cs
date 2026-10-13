using System.Text.Json.Nodes;
using Duplex.Tests.Support;
using static Duplex.Tests.Support.ScenarioSetup;

namespace Duplex.Tests;

public class ScenarioTests
{
    [Theory]
    [MemberData(nameof(GetApplicableScenarios), MemberType = typeof(ScenarioSetup))]
    public void Should_run_the_scenario(string name)
    {
        var run = () => Scenario.Run(name);

        var error = Record.Exception(run);
        Assert.Null(error);
    }

    [Fact]
    public void Should_run_every_scenario_of_a_provider_of_what_it_supports()
    {
        var provider = Directory
            .GetFiles(Scenario.Folder, "*.json")
            .Select(Scenario.Load)
            .Count(file =>
                (string?)file["peer"] == "provider"
                && (string?)file["protocol"] is null or "calculator"
                && file["major"] is null
            );

        var applicable = Scenario.GetApplicable().Count();

        Assert.InRange(applicable, 18, provider);
    }

    [Fact]
    public void Should_skip_a_scenario_requiring_what_it_leaves_out()
    {
        var file = JsonNode
            .Parse("""{"peer": "provider", "requires": ["responses"], "steps": []}""")!
            .AsObject();

        var applicable = Scenario.IsApplicable(file);

        Assert.False(applicable);
    }
}
