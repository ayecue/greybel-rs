using System.Text.Json.Nodes;

namespace Duplex.Tests.Support;

internal static class ScenarioSetup
{
    public static TheoryData<string> GetApplicableScenarios() => new(Scenario.GetApplicable());
}
