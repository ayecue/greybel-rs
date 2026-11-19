using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Game;

/// <summary>Game run settings: work location, BepInEx version, ports (0 selects a free port), and boot timeout.</summary>
internal sealed record GameOptions(
    WorkFolder WorkFolder,
    BepInExMajor Major,
    int Port,
    int TestPort,
    TimeSpan Timeout
);
