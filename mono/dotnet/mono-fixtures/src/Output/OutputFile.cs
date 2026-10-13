namespace MonoFixtures.Output;

internal static class OutputFile
{
    public static void WriteText(string path, string text)
    {
        CreateDirectory(path);
        File.WriteAllText(path, text, new UTF8Encoding(false));
    }

    public static void WriteBytes(string path, byte[] bytes)
    {
        CreateDirectory(path);
        File.WriteAllBytes(path, bytes);
    }

    public static void Copy(Stream source, string path)
    {
        CreateDirectory(path);
        using Stream file = File.Create(path);
        source.CopyTo(file);
    }

    private static void CreateDirectory(string path) =>
        Directory.CreateDirectory(Path.GetDirectoryName(path));
}
