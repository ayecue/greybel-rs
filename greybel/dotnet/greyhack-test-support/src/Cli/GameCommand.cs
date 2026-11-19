using System.CommandLine;
using GreyHackTestSupport.Game;
using GreyHackTestSupport.Session;
using GreyHackTestSupport.Setup;

namespace GreyHackTestSupport.Cli;

/// <summary>Implements <c>game run|start|host|stop</c> for tests that control the game through the message hook.</summary>
internal static class GameCommand
{
    private static readonly TimeSpan DefaultIdleTimeout = TimeSpan.FromMinutes(15);

    private static readonly Option<int> Bepinex = CreateBepinex();

    private static readonly Option<int> Port = new("--port")
    {
        Description = "The message hook's port; 0 picks a free one",
        DefaultValueFactory = _ => 0,
    };

    private static readonly Option<int> TestPort = new("--test-port")
    {
        Description = "The test-only setup/query protocol's port; 0 picks a free one",
        DefaultValueFactory = _ => 0,
    };

    private static readonly Option<DirectoryInfo?> WorkFolderOption = new("--work-folder")
    {
        Description =
            "Where the clone of the game, the downloads, the plugins' builds and the game's log go; "
            + "target/greyhack-game-<bepinex> without it",
    };

    private static readonly Option<int> Timeout = new("--timeout")
    {
        Description = "Seconds the game has to load single player and boot",
        DefaultValueFactory = _ => 120,
    };

    private static readonly Option<int> IdleTimeout = new("--idle-timeout")
    {
        Description = "Seconds a shared session may stay unused before it stops",
        DefaultValueFactory = _ => (int)DefaultIdleTimeout.TotalSeconds,
    };

    private static readonly Option[] GameOptionTable =
    [
        Bepinex,
        Port,
        TestPort,
        WorkFolderOption,
        Timeout,
    ];

    public static Command Create() =>
        new("game", "The game itself, for tests that drive it through the message hook")
        {
            CreateCommand(
                "run",
                "Run the installed game (GREY_HACK_PATH, else Steam's) headless with the message hook, until standard "
                    + "input ends: prints {\"port\":N,\"testPort\":N,\"log\":\"<path>\"} once the player's desktop has "
                    + "booted and both listeners are ready",
                GameOptionTable,
                result =>
                {
                    using GameInstance game = GameInstance.Start(ReadOptions(result));
                    Print(game.Connection.ToJson());
                    return game.RunUntilInputEnds();
                }
            ),
            CreateCommand(
                "start",
                "Start or reuse the shared game session and print its ports",
                [.. GameOptionTable, IdleTimeout],
                result =>
                    GameSession.Start(
                        ReadOptions(result),
                        TimeSpan.FromSeconds(result.GetValue(IdleTimeout)),
                        Console.Out
                    )
            ),
            CreateCommand(
                "host",
                "Run a shared session until it is stopped or idle",
                [.. GameOptionTable, IdleTimeout],
                result =>
                {
                    using GameInstance game = GameInstance.Start(ReadOptions(result));
                    Print(game.Connection.ToJson());
                    return game.RunAsSession(TimeSpan.FromSeconds(result.GetValue(IdleTimeout)));
                }
            ),
            CreateCommand(
                "stop",
                "Stop the shared game session",
                [Bepinex, WorkFolderOption],
                result => GameSession.Stop(ReadWorkFolder(result))
            ),
        };

    private static Option<int> CreateBepinex()
    {
        Option<int> bepinex = new("--bepinex")
        {
            Description = "The BepInEx major to run the game with, 5 or 6",
            DefaultValueFactory = _ => 5,
        };
        bepinex.Validators.Add(result =>
        {
            if (!Enum.IsDefined((BepInExMajor)result.GetValueOrDefault<int>()))
            {
                result.AddError("BepInEx 5 or 6");
            }
        });
        return bepinex;
    }

    private static Command CreateCommand(
        string name,
        string description,
        Option[] options,
        Func<ParseResult, int> action
    )
    {
        var command = new Command(name, description);
        foreach (Option option in options)
        {
            command.Options.Add(option);
        }
        command.SetAction(result => CommandAction.Run(() => action(result)));
        return command;
    }

    private static GameOptions ReadOptions(ParseResult result) =>
        new(
            ReadWorkFolder(result),
            (BepInExMajor)result.GetValue(Bepinex),
            result.GetValue(Port),
            result.GetValue(TestPort),
            TimeSpan.FromSeconds(result.GetValue(Timeout))
        );

    private static WorkFolder ReadWorkFolder(ParseResult result) =>
        WorkFolder.FromOption(
            result.GetValue(WorkFolderOption),
            (BepInExMajor)result.GetValue(Bepinex)
        );

    private static void Print(string line)
    {
        Console.Out.WriteLine(line);
        Console.Out.Flush();
    }
}
