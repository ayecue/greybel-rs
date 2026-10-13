using System.Text;

namespace GreyHackUtil.Protocol;

public sealed class CodeWriter(string indentation = "    ")
{
    private readonly StringBuilder output = new();
    private int depth;

    public void WriteLine(string text = "")
    {
        if (text.Length > 0)
        {
            for (int i = 0; i < depth; i++)
            {
                output.Append(indentation);
            }
            output.Append(text);
        }
        output.Append('\n');
    }

    public IDisposable OpenBlock(string open, string close)
    {
        WriteLine(open);
        depth++;
        return new Closer(() =>
        {
            depth--;
            WriteLine(close);
        });
    }

    public override string ToString() => output.ToString();

    private sealed class Closer(Action close) : IDisposable
    {
        public void Dispose() => close();
    }
}
