using System.Text;

namespace GreyHackTestSupport.Plugin.Bridge.Scripts;

/// <summary>Captures program terminal output.</summary>
internal sealed class Output
{
    private readonly StringBuilder text = new StringBuilder();

    /// <summary>True after any print operation, including one producing empty lines.</summary>
    private bool hasLines;

    /// <summary>Tracks replacement of printed lines (<c>TerminalListAdapter.AddText</c>).</summary>
    public void Print(string line, bool replace)
    {
        if (replace)
        {
            text.Clear();
        }
        else if (hasLines)
        {
            text.Append('\n');
        }

        text.Append(line);
        hasLines = true;
    }

    public void Clear()
    {
        text.Clear();
        hasLines = false;
    }

    public override string ToString()
    {
        return text.ToString();
    }
}
