#nullable enable
namespace Duplex;

public readonly struct Optional<T>
{
    private readonly T value;

    public Optional(T value)
    {
        this.value = value;
        HasValue = true;
    }

    public bool HasValue { get; }

    public T Value =>
        HasValue ? value : throw new System.InvalidOperationException("the field was left out");

    public static implicit operator Optional<T>(T value) => new Optional<T>(value);

    public override string ToString() => HasValue ? value?.ToString() ?? "null" : "absent";
}
