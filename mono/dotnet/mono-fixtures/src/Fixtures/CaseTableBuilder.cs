using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal sealed class CaseTableBuilder : IFixtureBuilder
{
    public string CrateName => "mono-string";

    public void Write(Crate crate)
    {
        TextInfo info = CultureInfo.InvariantCulture.TextInfo;
        var rust = new StringBuilder();
        AppendTable(rust, "UPPER", info.ToUpper);
        rust.Append('\n');
        AppendTable(rust, "LOWER", info.ToLower);
        OutputFile.WriteText(crate.GetPath("src", "case", "data.rs"), rust.ToString());
    }

    private static void AppendTable(StringBuilder rust, string name, Func<char, char> map)
    {
        var pairs = new List<string>();
        for (int unit = 0; unit <= 0xFFFF; unit++)
        {
            char mappedUnit = map((char)unit);
            if (mappedUnit != unit)
            {
                pairs.Add(
                    string.Format(
                        CultureInfo.InvariantCulture,
                        "(0x{0:X4}, 0x{1:X4})",
                        unit,
                        (int)mappedUnit
                    )
                );
            }
        }
        rust.Append("#[rustfmt::skip]\n");
        rust.Append("pub(super) const ")
            .Append(name)
            .Append(": [(u16, u16); ")
            .Append(pairs.Count)
            .Append("] = [\n");
        for (int i = 0; i < pairs.Count; i += 5)
        {
            rust.Append("    ").Append(string.Join(", ", pairs.Skip(i).Take(5))).Append(",\n");
        }
        rust.Append("];\n");
    }
}
