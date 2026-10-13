#nullable enable
namespace Duplex;

public sealed class ChannelOptions
{
    public const long DefaultMaxMessage = 64L * 1024 * 1024;

    public const int DefaultMaxQueue = 2048;

    public HealthCheck? HealthCheck { get; set; }

    public bool WithoutHealthCheck { get; set; }

    public int MaxQueue { get; set; } = DefaultMaxQueue;

    public long? MaxMessage { get; set; }

    public bool Unvalidated { get; set; }
}

internal sealed class ChannelSettings
{
    public ChannelSettings(ChannelOptions options, Flavour? flavour)
    {
        if (
            !options.WithoutHealthCheck
            && (flavour == null || !flavour.TurnsOffHealthCheck || options.HealthCheck != null)
        )
        {
            HealthCheck = options.HealthCheck ?? flavour?.HealthCheck ?? HealthCheck.Default;
        }
        MaxQueue = options.MaxQueue;
        MaxMessage = options.MaxMessage ?? flavour?.MaxMessage ?? ChannelOptions.DefaultMaxMessage;
        Validating = !options.Unvalidated;
    }

    public HealthCheck? HealthCheck { get; }

    public int MaxQueue { get; }

    public long MaxMessage { get; }

    public bool Validating { get; }
}
