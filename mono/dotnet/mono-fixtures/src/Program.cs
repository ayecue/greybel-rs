using MonoFixtures.Fixtures;
using MonoFixtures.Output;

CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;

IFixtureBuilder[] all =
[
    new NumberFixtureBuilder(),
    new CaseTableBuilder(),
    new StringFixtureBuilder(),
    new CollationFixtureBuilder(),
    new DictionaryFixtureBuilder(),
];
foreach (IFixtureBuilder fixtures in all)
{
    fixtures.Write(new Crate(fixtures.CrateName));
}
