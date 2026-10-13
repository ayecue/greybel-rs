using System.Text;

namespace GreyHackUtil.Assets;

internal static class NameShuffle
{
    public static byte[] Shuffle(string name, byte[] contents)
    {
        string text = Encoding.UTF8.GetString(contents);
        bool endsWithBreak = text.EndsWith('\n');
        string[] lines = (endsWithBreak ? text[..^1] : text).Split('\n');
        ulong state = GetSeed(name);
        for (int index = lines.Length - 1; index > 0; index--)
        {
            int other = (int)(Next(ref state) % (ulong)(index + 1));
            (lines[index], lines[other]) = (lines[other], lines[index]);
        }
        return Encoding.UTF8.GetBytes(string.Join('\n', lines) + (endsWithBreak ? "\n" : ""));
    }

    private static ulong GetSeed(string name)
    {
        ulong hash = 0xcbf29ce484222325;
        foreach (byte value in Encoding.UTF8.GetBytes(name))
        {
            hash = (hash ^ value) * 0x100000001b3;
        }
        return hash;
    }

    private static ulong Next(ref ulong state)
    {
        state += 0x9e3779b97f4a7c15;
        ulong bits = state;
        bits = (bits ^ (bits >> 30)) * 0xbf58476d1ce4e5b9;
        bits = (bits ^ (bits >> 27)) * 0x94d049bb133111eb;
        return bits ^ (bits >> 31);
    }
}
