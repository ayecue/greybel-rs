using GreyHackTestSupport.Plugin.Bridge.Fixtures;
using GreyHackTestSupport.Plugin.Bridge.Scripts;
using GreyHackTestSupport.Plugin.Bridge.Setup;
using GreyHackTestSupport.Plugin.Bridge.State;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Connection;

/// <summary>Dispatches each test-connection request to its handler with the reply destination.</summary>
internal sealed class PluginRouter(
    TestConnection connection,
    MainThread main,
    PlayerSetup setup,
    NetworkJoin join,
    ScriptRunner scripts,
    StateReader stateReader,
    NetworkFixture networks,
    KernelFixture kernels,
    AccountFixture accounts,
    EconomyFixture economy,
    NpcFixture npcs,
    ClockFixture clock,
    TrafficFixture traffic,
    ApplianceFixture appliances,
    ComputerFixture computer,
    RebootFixture reboot,
    ExploitShopFixture exploitShop,
    BankFixture bank,
    MissionFixture missions,
    GuildFixture guilds,
    ZeroDayFixture zeroDay
) : TestSupportPluginRouter
{
    public override void OnSetupPlayerRpc(SetupPlayerRpc message) =>
        setup.ReplaceComputer(From(message.Request, message));

    public override void OnConnectNetworkRpc(ConnectNetworkRpc message) =>
        join.ConnectNetwork(From(message.Request, message), message);

    public override void OnWriteFileRpc(WriteFileRpc message) =>
        Respond(message.Request, message, () => computer.WriteFile(message));

    public override void OnExecuteRpc(ExecuteRpc message) =>
        scripts.Execute(From(message.Request, message), message);

    public override void OnExecuteInTerminalRpc(ExecuteInTerminalRpc message) =>
        scripts.ExecuteInTerminal(From(message.Request, message), message);

    public override void OnExecuteInteractiveRpc(ExecuteInteractiveRpc message) =>
        scripts.ExecuteInteractive(From(message.Request, message), message);

    public override void OnExecuteUntilInputRpc(ExecuteUntilInputRpc message) =>
        scripts.ExecuteUntilInput(From(message.Request, message), message);

    public override void OnTypeInputRpc(TypeInputRpc message) =>
        scripts.SendInput(From(message.Request, message), message.Input);

    public override void OnExecutePairRpc(ExecutePairRpc message) =>
        scripts.ExecutePair(From(message.Request, message), message);

    public override void OnQueryPlayerRpc(QueryPlayerRpc message) =>
        Respond(message.Request, message, stateReader.ReadPlayer);

    public override void OnQueryFileRpc(QueryFileRpc message) =>
        Respond(message.Request, message, () => stateReader.ReadFile(message));

    public override void OnQueryNetworkRpc(QueryNetworkRpc message) =>
        Respond(message.Request, message, stateReader.ReadNetwork);

    public override void OnCreateNetworkRpc(CreateNetworkRpc message) =>
        Respond(message.Request, message, () => networks.Create(message));

    public override void OnCreateFixtureNetworkRpc(CreateFixtureNetworkRpc message) =>
        Respond(message.Request, message, () => networks.CreateFixture(message));

    public override void OnSpawnNetworkRpc(SpawnNetworkRpc message) =>
        Respond(message.Request, message, () => networks.Spawn(message));

    public override void OnDisableNetworkRpc(DisableNetworkRpc message) =>
        Respond(message.Request, message, () => networks.Disable(message));

    public override void OnForwardServiceRpc(ForwardServiceRpc message) =>
        Respond(message.Request, message, () => networks.ForwardService(message));

    public override void OnInstallLibraryRpc(InstallLibraryRpc message) =>
        Respond(message.Request, message, () => networks.InstallLibrary(message));

    public override void OnInstallFixtureLibraryRpc(InstallFixtureLibraryRpc message) =>
        Respond(message.Request, message, () => networks.InstallFixture(message));

    public override void OnAllowFirewallRpc(AllowFirewallRpc message) =>
        Respond(message.Request, message, () => networks.AllowFirewall(message));

    public override void OnPrimeFloodRpc(PrimeFloodRpc message) =>
        Respond(message.Request, message, () => networks.PrimeFlood(message));

    public override void OnSetRouterKernelRequirementRpc(SetRouterKernelRequirementRpc message) =>
        Respond(message.Request, message, () => kernels.SetRequirement(message));

    public override void OnWriteRouterKernelRpc(WriteRouterKernelRpc message) =>
        Respond(message.Request, message, () => kernels.Write(message));

    public override void OnSetupDebugRpc(SetupDebugRpc message) =>
        Respond(message.Request, message, accounts.CreateDebug);

    public override void OnSetupCtfRpc(SetupCtfRpc message) =>
        Respond(message.Request, message, () => accounts.CreateCtf(message));

    public override void OnRateCtfRpc(RateCtfRpc message) =>
        Respond(message.Request, message, () => accounts.WriteCtfScore(message));

    public override void OnSpawnCtfNetworkRpc(SpawnCtfNetworkRpc message) =>
        Respond(message.Request, message, () => networks.SpawnCtf(message));

    public override void OnConfigureComputerRpc(ConfigureComputerRpc message) =>
        Respond(message.Request, message, () => computer.Configure(message));

    public override void OnSetupMailRpc(SetupMailRpc message) =>
        Respond(message.Request, message, () => accounts.CreateMail(message));

    public override void OnSetupBlockchainRpc(SetupBlockchainRpc message) =>
        Respond(message.Request, message, () => economy.CreateBlockchain(message));

    public override void OnSetupWalletRpc(SetupWalletRpc message) =>
        Respond(message.Request, message, () => economy.CreateWallet(message));

    public override void OnConfigureMailRpc(ConfigureMailRpc message) =>
        Respond(message.Request, message, () => accounts.ConfigureMail(message));

    public override void OnDeleteCtfRpc(DeleteCtfRpc message) =>
        Respond(message.Request, message, () => accounts.DeleteCtf(message));

    public override void OnConfigureApplianceRpc(ConfigureApplianceRpc message) =>
        Respond(message.Request, message, () => appliances.Configure(message));

    public override void OnSetupForeignCoinRpc(SetupForeignCoinRpc message) =>
        Respond(message.Request, message, () => economy.CreateForeignCoin(message));

    public override void OnConfigureEconomyRpc(ConfigureEconomyRpc message) =>
        Respond(message.Request, message, () => economy.Configure(message));

    public override void OnPromoteAdminRpc(PromoteAdminRpc message) =>
        Respond(message.Request, message, () => npcs.PromoteAdmin(message));

    public override void OnFireNpcRpc(FireNpcRpc message) =>
        Respond(message.Request, message, () => npcs.FireNpc(message));

    public override void OnRefreshNpcAccountRpc(RefreshNpcAccountRpc message) =>
        Respond(message.Request, message, () => npcs.RefreshAccount(message));

    public override void OnConfigZeroDayInfoRpc(ConfigZeroDayInfoRpc message) =>
        Respond(message.Request, message, () => npcs.WriteZeroDayInfo(message));

    public override void OnSetPhoneAlarmRpc(SetPhoneAlarmRpc message) =>
        Respond(message.Request, message, () => npcs.SetPhoneAlarm(message));

    public override void OnForceAdminLoginRpc(ForceAdminLoginRpc message) =>
        Respond(message.Request, message, () => npcs.ForceAdminLogin(message));

    public override void OnReportLibraryUpdateRpc(ReportLibraryUpdateRpc message) =>
        Respond(message.Request, message, () => npcs.ReportLibraryUpdate(message));

    public override void OnHireRentalAdminRpc(HireRentalAdminRpc message) =>
        Respond(message.Request, message, () => npcs.HireRentalAdmin(message));

    public override void OnFreezeClockRpc(FreezeClockRpc message) =>
        Respond(message.Request, message, clock.Stop);

    public override void OnAdvanceClockRpc(AdvanceClockRpc message) =>
        Respond(message.Request, message, () => clock.Advance(message));

    public override void OnSetupTrafficRpc(SetupTrafficRpc message) =>
        Respond(message.Request, message, traffic.Create);

    public override void OnTrafficAccidentRpc(TrafficAccidentRpc message) =>
        Respond(message.Request, message, () => traffic.RunAccident(message));

    public override void OnResetTrafficCooldownRpc(ResetTrafficCooldownRpc message) =>
        Respond(message.Request, message, traffic.ResetCooldown);

    public override void OnQueryExploitsRpc(QueryExploitsRpc message) =>
        Respond(message.Request, message, () => exploitShop.ReadExploits(message));

    public override void OnReleaseLibraryVersionRpc(ReleaseLibraryVersionRpc message) =>
        Respond(message.Request, message, () => exploitShop.ReleaseLibraryVersion(message));

    public override void OnQueryWordListsRpc(QueryWordListsRpc message) =>
        Respond(message.Request, message, missions.ReadWordLists);

    public override void OnMissionRpc(MissionRpc message) =>
        Respond(message.Request, message, () => missions.ReadMission(message));

    public override void OnZeroDayRumorRpc(ZeroDayRumorRpc message) =>
        Respond(message.Request, message, () => zeroDay.Rumor(message));

    public override void OnZeroDayStateRpc(ZeroDayStateRpc message) =>
        Respond(message.Request, message, () => zeroDay.State(message));

    public override void OnCreateZeroDayChainRpc(CreateZeroDayChainRpc message) =>
        Respond(message.Request, message, () => zeroDay.CreateZeroDayChain(message));

    public override void OnCreateProceduralMissionRpc(CreateProceduralMissionRpc message) =>
        Respond(message.Request, message, () => zeroDay.CreateProceduralMission(message));

    public override void OnUnlockDebugLibraryRpc(UnlockDebugLibraryRpc message) =>
        Respond(message.Request, message, () => zeroDay.Unlock(message));

    public override void OnBuyExploitRpc(BuyExploitRpc message) =>
        exploitShop.CreatePurchase(From(message.Request, message), message);

    public override void OnLockApplianceRpc(LockApplianceRpc message) =>
        Respond(message.Request, message, () => appliances.LockAppliance(message));

    public override void OnRebootPlayerRpc(RebootPlayerRpc message) =>
        reboot.Restart(From(message.Request, message), message);

    public override void OnRentNetworkRpc(RentNetworkRpc message) =>
        Respond(message.Request, message, () => bank.CreateRental(message));

    public override void OnRentalAdminServiceRpc(RentalAdminServiceRpc message) =>
        Respond(message.Request, message, () => bank.WriteRentalAdmin(message));

    public override void OnRunPaymentsRpc(RunPaymentsRpc message) =>
        Respond(message.Request, message, bank.RunPayments);

    public override void OnSetPlayerMoneyRpc(SetPlayerMoneyRpc message) =>
        Respond(message.Request, message, () => bank.SetMoney(message));

    public override void OnSetupGuildRpc(SetupGuildRpc message) =>
        Respond(message.Request, message, () => guilds.CreateGuild(message));

    public override void OnQueryGuildsRpc(QueryGuildsRpc message) =>
        Respond(message.Request, message, () => guilds.Read(message));

    public override void OnQueryTracesRpc(QueryTracesRpc message) =>
        Respond(message.Request, message, guilds.ReadTraces);

    public override void OnEndGameRpc(EndGameRpc message) =>
        Respond(message.Request, message, () => guilds.EndGame(message));

    public override void OnRunPaymentRpc(RunPaymentRpc message) =>
        Respond(message.Request, message, () => bank.RunPayment(message));

    /// <summary>Executes <paramref name="operation"/> on the main thread and replies with its result.</summary>
    private void Respond(uint request, Duplex.Message message, System.Func<Reply> operation) =>
        main.Respond(From(request, message), operation);

    /// <summary>Identifies the reply destination for request <paramref name="request"/> and its <paramref name="message"/>.</summary>
    private Requester From(uint request, Duplex.Message message)
    {
        UnityEngine.Debug.Log($"GreyHackTestSupport - request {request}: {message.GetType().Name}");
        return new Requester(connection, request);
    }
}
