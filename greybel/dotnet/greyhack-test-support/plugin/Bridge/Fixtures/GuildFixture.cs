using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Makes guilds, their companies and CrownShield with the game's own types, and reads them back.</summary>
internal sealed class GuildFixture(Player player) : IFixture
{
    /// <summary>Guilds this fixture added, which the next player setup removes.</summary>
    private readonly List<string> guilds = new List<string>();

    /// <summary>Networks this fixture protected with CrownShield, which the next player setup unprotects.</summary>
    private readonly List<string> shields = new List<string>();

    /// <summary>Companies whose shares this fixture bought, as (company address, bank account) pairs.</summary>
    private readonly List<(string CompanyIp, string BankAccount)> purchases =
        new List<(string, string)>();

    public void Reset()
    {
        foreach ((string companyIp, string bankAccount) in purchases)
        {
            RestoreCompanyShares(companyIp, bankAccount);
        }
        purchases.Clear();
        // Every guild in a test copy is a test's, including ones an earlier run created: another guild in the ranking
        // makes the game revoke a test guild's Crown at its next evaluation (`Guilds.EvaluateCrownRanking`, at most
        // 60 s later), which removes its CrownShield.
        foreach (string name in guilds.Union(Guilds.Singleton.guilds.Keys).ToList())
        {
            Database.Singleton.DeleteCrownShieldNetwork(name);
            Guilds.Singleton.guilds.Remove(name);
        }
        guilds.Clear();
        shields.Clear();
        Database.Singleton.SyncGuilds();
    }

    /// <summary>
    /// Adds a guild with the game's guild types: the shares of the company at <c>CompanyIp</c> are bought for its bank
    /// account as <c>StocksCompany.BuyStocksCompany</c> does, without a price, and the company is registered as one the
    /// guild invested in as <c>BankHelperServer.PurchaseStockServerRpc</c> does.
    /// </summary>
    public Reply CreateGuild(SetupGuildRpc request)
    {
        Guilds.Guild guild = (Guilds.Guild)Activator.CreateInstance(typeof(Guilds.Guild), true);
        guild.name = request.Name;
        guild.bankAccount = request.BankAccount;
        guild.totalExp = (int)request.TotalExp;
        guild.purchasedInvestmentLicensePerks = (int)request.InvestmentPerks;
        guild.purchasedAdminRestorePerks = (int)request.AdminRestorePerks;
        guild.hasCrown = request.Crown;
        if (request.Join)
        {
            Guilds.Member member = new Guilds.Member
            {
                name = player.User,
                playerID = player.Server.GetPlayerID(),
                isOwner = true,
            };
            guild.owner = member;
            guild.members.Add(member);
        }
        Guilds.Singleton.guilds[request.Name] = guild;
        guilds.Add(request.Name);

        if (request.CompanyIp.Length > 0)
        {
            CreateCompanyShares(guild, request.CompanyIp);
            purchases.Add((request.CompanyIp, guild.bankAccount));
        }
        if (request.AdminRestoreOff && !guild.SetAdminRestoreEnabled(request.CompanyIp, false))
        {
            throw new InvalidOperationException(
                "The guild cannot turn Admin Restore off for " + request.CompanyIp
            );
        }
        if (request.ShieldIp.Length > 0)
        {
            guild.crownShieldPublicIp = request.ShieldIp;
            Database.Singleton.AddCrownShieldNetwork(request.ShieldIp, request.Name);
            shields.Add(request.Name);
        }
        Database.Singleton.SyncGuilds();
        return new Reply.Applied();
    }

    /// <summary>Reads the guilds, the companies requested and the protected networks.</summary>
    public Reply Read(QueryGuildsRpc request)
    {
        string playerId = player.Server.GetPlayerID();
        List<GuildSnapshot> snapshots = Guilds
            .Singleton.guilds.Values.Select(guild =>
            {
                Guilds.Member member = guild.GetMemberByPlayerID(playerId);
                return new GuildSnapshot(
                    guild.name,
                    guild.bankAccount,
                    (uint)guild.totalExp,
                    (uint)guild.purchasedInvestmentLicensePerks,
                    guild.hasCrown,
                    guild.crownShieldPublicIp ?? "",
                    member != null,
                    member?.isOwner ?? false,
                    (uint)guild.purchasedAdminRestorePerks,
                    guild.adminRestoreDisabledCompanies.ToList()
                );
            })
            .ToList();
        List<CompanySnapshot> companies = request
            .CompanyIps.Select(ip => ReadCompany(ip))
            .Where(company => company != null)
            .ToList();
        List<ShieldSnapshot> protectedNetworks = request
            .CompanyIps.Concat(snapshots.Select(guild => guild.CrownShieldPublicIp))
            .Where(ip => ip.Length > 0)
            .Distinct()
            .Where(ip => Database.Singleton.IsCrownShieldProtected(ip))
            .Select(ip => new ShieldSnapshot(
                ip,
                snapshots.FirstOrDefault(guild => guild.CrownShieldPublicIp == ip)?.Name ?? ""
            ))
            .ToList();

        return new Reply.Guilds(new GuildsReply(snapshots, companies, protectedNetworks));
    }

    /// <summary>
    /// Gives the player strike <c>Strikes</c> as the end of an active trace does: the strikes the player already has
    /// are set as <c>Database.GameOverStrike</c> stores them, then <c>PlayerHelperServer.ActiveGameOver</c> runs for
    /// the company at <c>CompanyIp</c>, which ends the game on the third (<c>Database.GameOver</c>).
    /// </summary>
    public Reply EndGame(EndGameRpc request)
    {
        PlayerHelperServer helper = player.Server.playerHelper;
        PlayerComputer computer = player.Computer;
        int before = (int)request.Strikes - 1;
        Router company =
            ServerMap.Singleton.GetRouter(new IP(request.CompanyIp), false)
            ?? throw new InvalidOperationException("No company at " + request.CompanyIp);

        GameField.GameOverStrikes.SetValue(helper, before);
        Database.Singleton.GameOverStrike(computer, before);
        helper.ActiveGameOver(company, PlayerUtils.Trace.ACTIVE);

        Router endedCompany = ServerMap.Singleton.GetRouter(new IP(request.CompanyIp), false) ?? company;

        return new Reply.GameEnded(
            new GameEndedReply(
                player.Server.GetComputer() != null,
                BankFixture.Read(player.Server.GetPlayerID()),
                endedCompany.GetNetworkLan().disabledNetwork,
                endedCompany.IsHomeNetwork(),
                helper.rentalIPs
                    .Select(entry => new RentalStateSnapshot(entry.Key, entry.Value))
                    .ToList()
            )
        );
    }

    /// <summary>Reads the active traces, the passive trace's queue and the strikes of the player.</summary>
    public Reply ReadTraces()
    {
        PlayerHelperServer helper = player.Server.playerHelper;
        IEnumerable<ActiveTraceSystem.ActiveTrace> active = (
            (IEnumerable<ActiveTraceSystem.ActiveTrace>)
                GameField.ActiveTraces.GetValue(helper.activeTrace)
        ).ToList();
        IEnumerable<string> passive = (
            (IEnumerable<string>)GameField.PassiveQueue.GetValue(helper.passiveTrace)
        ).ToList();

        return new Reply.Traces(
            new TracesReply(
                active.Select(trace => ReadTraceComputer(trace.GetID())).ToList(),
                passive.Select(ReadTraceComputer).ToList(),
                (uint)(int)GameField.GameOverStrikes.GetValue(helper)
            )
        );
    }

    private static TraceComputer ReadTraceComputer(string netId)
    {
        Computer computer = ServerMap.Singleton.GetRemoteComputer(netId);
        return new TraceComputer(computer.GetPublicIP(), computer.GetLocalIP());
    }

    /// <summary>
    /// Gives the shares bought for <paramref name="bankAccount"/> back to the company as
    /// <c>Database.RestoreStocksToCompany</c> does, since the game stores its companies between tests.
    /// </summary>
    private static void RestoreCompanyShares(string companyIp, string bankAccount)
    {
        Router router = ServerMap.Singleton.GetRouter(new IP(companyIp), false);
        if (router == null)
        {
            return;
        }
        StocksCompany company = StocksCompany.GetCompany(router);
        company.RestoreStocksToCompany(bankAccount);
        // `StocksCompany.SyncMarketData` writes only the listed kinds, and a test may buy shares of any kind.
        Database.Singleton.SyncStocks(
            router.GetPublicIP(),
            company,
            router.GetNetworkLan().GetCurrentSharePrice(router)
        );
    }

    /// <summary>
    /// Buys all the available shares of the company at <c>companyIp</c> for the guild's account. When that account is
    /// the player's own, the purchase is the player's, as <c>BankHelperServer.PurchaseStockServerRpc</c> makes it: it
    /// lists the company among the player's (<c>PlayerHelperServer.AddStocksCompany</c>), which
    /// <c>Database.RestoreStocksToCompany</c> goes through when the game ends.
    /// </summary>
    private void CreateCompanyShares(Guilds.Guild guild, string companyIp)
    {
        Router router = ServerMap.Singleton.GetRouter(new IP(companyIp));
        BankAccount buyer = Database.Singleton.GetBankAccount(guild.bankAccount, out string error);
        if (buyer == null)
        {
            throw new InvalidOperationException(error);
        }
        StocksCompany company = StocksCompany.GetCompany(router);
        int price = router.GetNetworkLan().GetCurrentSharePrice(router);
        string failure = company.BuyStocksCompany(
            buyer,
            company.GetAvailableShares(),
            router.GetDomain(),
            0,
            price,
            true,
            true,
            buyer.account == player.Server.GetComputer().GetUserBank()?.userName
                ? player.Server
                : null
        );
        if (failure.Length > 0)
        {
            throw new InvalidOperationException(failure);
        }
        guild.RegisterInvestedCompany(companyIp);
    }

    private static CompanySnapshot ReadCompany(string companyIp)
    {
        StocksCompany company = Database.Singleton.GetStocks(companyIp);
        if (company == null)
        {
            return null;
        }
        Router router = ServerMap.Singleton.GetRouter(new IP(companyIp));
        IDictionary entries = (IDictionary)GameField.StocksHolders.GetValue(company);
        List<HolderSnapshot> holders = new List<HolderSnapshot>();
        foreach (DictionaryEntry entry in entries)
        {
            holders.Add(
                new HolderSnapshot(
                    (string)entry.Key,
                    (uint)((StocksCompany.HelperStocks)entry.Value).numShares
                )
            );
        }
        return new CompanySnapshot(
            companyIp,
            router.GetNetworkLan().GetSeed(),
            holders,
            Database.Singleton.GetStocks(companyIp).HasDdosRankingPenalty()
        );
    }
}
