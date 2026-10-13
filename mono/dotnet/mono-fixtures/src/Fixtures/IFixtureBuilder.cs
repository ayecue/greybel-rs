using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal interface IFixtureBuilder
{
    string CrateName { get; }

    void Write(Crate crate);
}
