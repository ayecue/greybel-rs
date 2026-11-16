using System.Reflection;

namespace GreyHackTestSupport.Plugin.Bridge.Game;

/// <summary>Defines each private game field accessed by the bridge in one place.</summary>
internal static class GameField
{
    /// <summary>Smart-appliance override lock state.</summary>
    public static readonly FieldInfo ApplianceOverrideSettings = typeof(ConfigOS).GetField(
        "overrideSettings",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Smart-appliance power state.</summary>
    public static readonly FieldInfo AppliancePower = typeof(SmartAppConfigOS).GetField(
        "power",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Recovery time for a broken smart appliance.</summary>
    public static readonly FieldInfo ApplianceRecoveryTime = typeof(SmartAppConfigOS).GetField(
        "brokenUntil",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Smart-appliance target temperature.</summary>
    public static readonly FieldInfo ApplianceTemperature = typeof(SmartAppConfigOS).GetField(
        "temperature",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>World clock value.</summary>
    public static readonly FieldInfo ClockCurrentTime = typeof(Clock).GetField(
        "currentTime",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Next coin mining-round timestamp.</summary>
    public static readonly FieldInfo CoinNextCycle = typeof(BlockchainSystem.Coin).GetField(
        "nextCycle",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Runs a monthly bank payment as its timer does when the payment's date comes.</summary>
    public static readonly MethodInfo PaymentRunPayment = typeof(BankSubs.Payment).GetMethod(
        "RunPayment",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Real-time delay required between CTF account creations.</summary>
    public static readonly FieldInfo CtfCooldown = typeof(MissionsHelperServer).GetField(
        "ctfCooldown",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Memory zone address an exploit was made from.</summary>
    public static readonly FieldInfo ExploitMemZone = typeof(Exploit).GetField(
        "memZoneAddress",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Whether an exploit was made from a hidden memory zone of a replaced library version.</summary>
    public static readonly FieldInfo ExploitOnlyHide = typeof(Exploit).GetField(
        "onlyHide",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Script part an exploit's source is made from.</summary>
    public static readonly FieldInfo ExploitPart = typeof(Exploit).GetField(
        "partSelected",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Weakness an exploit was made from.</summary>
    public static readonly FieldInfo ExploitVulnerab = typeof(Exploit).GetField(
        "vulnerab",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Serialized file size.</summary>
    public static readonly FieldInfo FileSize = typeof(FileSystem.Fichero).GetField(
        "size",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Serialized file type.</summary>
    public static readonly FieldInfo FileType = typeof(FileSystem.Fichero).GetField(
        "typeFile",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>NPC surname owning a smartphone (`LanPcInfo.npcOwner`).</summary>
    public static readonly FieldInfo LanNpcOwner = typeof(LanPcInfo).GetField(
        "npcOwner",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Fresh LAN generation context used to add game-style child devices.</summary>
    public static readonly FieldInfo NetworkGlobalLan = typeof(NetworkLan).GetField(
        "globalLan",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Police vehicle plate, which NpcInfo has no public setter for.</summary>
    public static readonly FieldInfo NpcLicensePlate = typeof(NpcInfo).GetField(
        "licensePlate",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Exploit shelves indexed by library name and version.</summary>
    public static readonly FieldInfo VersionsExploits = typeof(VersionsControl).GetField(
        "exploits",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Library version number.</summary>
    public static readonly FieldInfo LibraryVersion = typeof(Library).GetField(
        "version",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Expiration of an NPC login forced by <c>Persona.ForceLogin</c>.</summary>
    public static readonly FieldInfo NpcForcedLoginEnd = typeof(NpcInfo).GetField(
        "dateForceLogin",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>NPC intelligence stat.</summary>
    public static readonly FieldInfo NpcIntelligence = typeof(NpcInfo).GetField(
        "inteligencia",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>NPC computer-experience stat.</summary>
    public static readonly FieldInfo NpcPcExperience = typeof(NpcInfo).GetField(
        "expPcs",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>NPC work-performance stat.</summary>
    public static readonly FieldInfo NpcWorkPerformance = typeof(NpcInfo).GetField(
        "workPerformance",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Router library versions indexed by library name.</summary>
    public static readonly FieldInfo RouterLibraryVersions =
        typeof(ServerMap.HelperLibVersions).GetField(
            "libVersions",
            BindingFlags.Instance | BindingFlags.NonPublic
        );

    /// <summary>Traffic-camera cooldown expiration.</summary>
    public static readonly FieldInfo TrafficCooldown = typeof(PlayerHelperServer).GetField(
        "cooldownTraffic",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>Last wallet password-reset timestamp.</summary>
    public static readonly FieldInfo WalletLastPasswordReset =
        typeof(BlockchainSystem.Wallet).GetField(
            "lastPassReset",
            BindingFlags.Instance | BindingFlags.NonPublic
        );

    /// <summary>Shares each bank account has of a company.</summary>
    public static readonly FieldInfo StocksHolders = typeof(StocksCompany).GetField(
        "stocks",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>The player's strikes.</summary>
    public static readonly FieldInfo GameOverStrikes = typeof(PlayerHelperServer).GetField(
        "gameOverCont",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>The active traces of the player.</summary>
    public static readonly FieldInfo ActiveTraces = typeof(ActiveTraceSystem).GetField(
        "activeTraces",
        BindingFlags.Instance | BindingFlags.NonPublic
    );

    /// <summary>The computers the passive trace has queued.</summary>
    public static readonly FieldInfo PassiveQueue = typeof(PassiveTrace).GetField(
        "nextRemoteNetID",
        BindingFlags.Instance | BindingFlags.NonPublic
    );
}
