using System;
using System.Net;
using System.Threading;
using GreyHackMessageHook;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Plugin.Bridge.Fixtures;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Plugin.Bridge.Scripts;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Plugin.Bridge.State;

namespace GreyHackTestSupport.Plugin.Bridge;

/// <summary>
/// Exposes a test-only protocol on <c>port</c> for fixture setup, state queries, and script execution. Every request
/// runs on the game main thread.
/// </summary>
internal sealed class TestSupportBridge : IDisposable
{
    private readonly TestSupportProxy proxy = new TestSupportProxy();
    private readonly MainThread main;
    private readonly NetworkJoin join;
    private readonly ScriptRunner scripts;
    private readonly ExploitShopFixture exploitShop;

    public TestSupportBridge(int port)
    {
        main = new MainThread(SynchronizationContext.Current);
        Player player = new Player();
        RouterKernel routerKernels = new RouterKernel(player);
        WorldRestore restore = new WorldRestore(player, routerKernels);
        FixtureStore store = new FixtureStore();
        AccountFixture accounts = new AccountFixture(player);
        NpcFixture npcs = new NpcFixture();
        BankFixture bank = new BankFixture(player);
        ClockFixture clock = new ClockFixture(npcs);
        TrafficFixture traffic = new TrafficFixture(player, clock);
        GuildFixture guilds = new GuildFixture(player);
        PlayerSetup setup = new PlayerSetup(
            main,
            player,
            restore,
            store,
            [clock, accounts, npcs, traffic, bank, guilds]
        );
        join = new NetworkJoin(main, player);
        scripts = new ScriptRunner(main, player);
        StateReader stateReader = new StateReader(player, accounts, traffic);
        NetworkFixture networks = new NetworkFixture(player, restore, store);
        KernelFixture kernels = new KernelFixture(routerKernels, restore, networks);
        EconomyFixture economy = new EconomyFixture(player);
        exploitShop = new ExploitShopFixture(main, player);
        MissionFixture missions = new MissionFixture(clock);

        Proxy.RegisterMessageListener(OnGameMessage);
        proxy.Start(
            new IPEndPoint(IPAddress.Loopback, port),
            connection => new PluginRouter(
                connection,
                main,
                setup,
                join,
                scripts,
                stateReader,
                networks,
                kernels,
                accounts,
                economy,
                npcs,
                clock,
                traffic,
                new ApplianceFixture(player),
                new ComputerFixture(player),
                new RebootFixture(main, player),
                exploitShop,
                bank,
                missions,
                guilds
            )
        );
    }

    public void Dispose()
    {
        Proxy.UnregisterMessageListener(OnGameMessage);
        proxy.Dispose();
    }

    /// <summary>Handles game messages from any thread, including script threads, on the main thread.</summary>
    private void OnGameMessage(Duplex.Message message)
    {
        main.Post(() =>
        {
            join.Handle(message);
            scripts.Handle(message);
            exploitShop.Handle(message);
        });
    }
}
