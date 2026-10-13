namespace MonoFixtures.Output;

internal sealed class JsonWriter
{
    private readonly StringBuilder _text = new();

    private readonly Stack<bool> _containsItems = new();

    public void WriteStartObject(string? name = null) => WriteStart(name, '{');

    public void WriteEndObject() => WriteEnd('}');

    public void WriteStartArray(string? name = null) => WriteStart(name, '[');

    public void WriteEndArray() => WriteEnd(']');

    public void WriteValue(string value)
    {
        WriteItemStart(null);
        _text.Append(value);
    }

    public void WriteProperty(string name, string value)
    {
        WriteItemStart(name);
        _text.Append(value);
    }

    public void WriteArray(string name, IEnumerable<string> items)
    {
        WriteStartArray(name);
        foreach (string item in items)
        {
            WriteValue(item);
        }
        WriteEndArray();
    }

    public void Save(string path)
    {
        if (_containsItems.Count > 0)
        {
            throw new InvalidOperationException(
                $"{path}: {_containsItems.Count} objects or arrays are not closed"
            );
        }
        OutputFile.WriteText(path, _text.Append('\n').ToString());
    }

    private void WriteStart(string? name, char bracket)
    {
        WriteItemStart(name);
        _text.Append(bracket);
        _containsItems.Push(false);
    }

    private void WriteEnd(char bracket)
    {
        _containsItems.Pop();
        WriteLineStart();
        _text.Append(bracket);
    }

    private void WriteItemStart(string? name)
    {
        if (_containsItems.Count > 0)
        {
            if (_containsItems.Pop())
            {
                _text.Append(',');
            }
            _containsItems.Push(true);
            WriteLineStart();
        }
        if (name != null)
        {
            _text.Append(Json.FormatString(name)).Append(": ");
        }
    }

    private void WriteLineStart() => _text.Append('\n').Append(' ', 2 * _containsItems.Count);
}
