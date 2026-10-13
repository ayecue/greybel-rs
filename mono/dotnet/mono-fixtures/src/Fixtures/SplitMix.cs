namespace MonoFixtures.Fixtures;

internal sealed class SplitMix(ulong seed)
{
    private ulong _state = seed;

    public ulong Next()
    {
        _state += 0x9E3779B97F4A7C15;
        ulong z = _state;
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9;
        z = (z ^ (z >> 27)) * 0x94D049BB133111EB;
        return z ^ (z >> 31);
    }

    public int NextInt(int max) => (int)(Next() % (ulong)max);

    public double NextDigits(int count)
    {
        ulong value = 0;
        for (int i = 0; i < count; i++)
        {
            value = value * 10 + (ulong)NextInt(10);
        }
        return value;
    }

    public double ApplyRandomSign(double value) => NextInt(2) == 0 ? value : -value;
}
