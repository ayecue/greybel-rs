using Duplex.Tests.Sensor;

namespace Duplex.Tests.Support;

internal static class TagSetup
{
    internal static IEnumerable<(string Name, object? Value)> ToPairs(IReadOnlyList<Tag> tags) =>
        tags.Select(tag => (tag.Name, tag.Value));
}
