using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;
using OS = Util.OS;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Controls network NPCs, administrator roles, dismissal, phone alarms, and logins.</summary>
internal sealed class NpcFixture : IFixture
{
    /// <summary>NPC, network, and account affected by <see cref="SetPhoneAlarm"/>.</summary>
    private NpcInfo alarmNpc;
    private Router alarmRouter;
    private Persona alarmPersona;

    public void Reset()
    {
        alarmNpc = null;
        alarmRouter = null;
        alarmPersona = null;
    }

    /// <summary>Grants the network administrator super-administrator stats (<c>NpcInfo.IsSuperAdmin</c>).</summary>
    public Reply PromoteAdmin(PromoteAdminRpc request)
    {
        Router router = GetNetwork(request.PublicIp);
        NpcInfo admin =
            router.GetNetworkLan().GetAdminNpcInfo()
            ?? throw new InvalidOperationException("No administrator at " + request.PublicIp);

        GameField.NpcIntelligence.SetValue(admin, 10);
        GameField.NpcPcExperience.SetValue(admin, 10);
        GameField.NpcWorkPerformance.SetValue(
            admin,
            Enum.GetValues(typeof(NpcInfo.WorkStatus)).Cast<NpcInfo.WorkStatus>().Max()
        );
        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    public Reply FireNpc(FireNpcRpc request)
    {
        Router router = GetNetwork(request.PublicIp);
        string error = router.GetNetworkLan().FireNpc(router, request.User);

        if (!string.IsNullOrEmpty(error))
        {
            throw new InvalidOperationException(error);
        }

        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    /// <summary>
    /// Has an NPC's workstation refresh its account of the NPC and saves it. The game stores that account as a copy
    /// of the NPC's state (<c>Persona.npcInfo</c>) that <c>Persona.NextCita</c> updates, run for every account of a
    /// computer by <c>Computer.NpcActions</c> when a player connects to it; until then <c>Database.GetMailAccount</c>
    /// and the shell logins read the copy of before a dismissal. Run on the workstation object itself and saved
    /// with <c>Database.SyncConfigOS</c>, so the refresh does not depend on which computer object a script call gets
    /// (<c>HelperScript.GetRemoteComputer</c> caches only one).
    /// </summary>
    public Reply RefreshAccount(RefreshNpcAccountRpc request)
    {
        Router router = GetNetwork(request.PublicIp);
        NpcInfo npc =
            router.GetNetworkLan().GetNpcInfo(request.User)
            ?? throw new InvalidOperationException("No NPC " + request.User);
        Computer workstation =
            router.GetComputer(npc.GetLocalIP())
            ?? throw new InvalidOperationException("No workstation for " + request.User);
        Persona account =
            workstation.GetPersona(npc.GetSurname(), true)
            ?? throw new InvalidOperationException("No account of " + request.User);

        account.NextCita();
        Database.Singleton.SyncConfigOS(workstation);
        return new Reply.Applied();
    }

    /// <summary>Gives NPCs a company stance and a zero-day contact as the zero-day rumor network does.</summary>
    public Reply WriteZeroDayInfo(ConfigZeroDayInfoRpc request)
    {
        Router router = GetNetwork(request.PublicIp);

        router.GetNetworkLan().ConfigZeroDayInfo();
        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    /// <summary>Uses the clock-program behavior to change an NPC phone alarm and resulting lateness.</summary>
    public Reply SetPhoneAlarm(SetPhoneAlarmRpc request)
    {
        if (request.Hour < 0 || request.Hour > 23 || request.Minute < 0 || request.Minute > 59)
        {
            throw new InvalidOperationException(
                "Error: Incorrect time. A 24-hour format is expected."
            );
        }
        Router router = GetNetwork(request.PublicIp);
        NpcInfo npc = router.GetNetworkLan().GetNpcInfo(request.User);
        alarmNpc = npc;
        alarmRouter = router;
        Computer workstation = router.GetComputer(npc.GetLocalIP());
        alarmPersona = workstation
            ?.GetConfigOS()
            .personas.FirstOrDefault(persona => persona.GetNombreUser(true) == request.User);
        if (alarmPersona == null && router.admin?.GetNombreUser(true) == request.User)
        {
            alarmPersona = router.admin;
        }
        AlarmConfig alarm = npc.GetAlarmConfig();
        alarm.SetAlarmConfig((int)request.Hour, (int)request.Minute, request.DeleteAlarm);
        npc.SetLateToWork();
        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    /// <summary>
    /// Advances the NPC affected by <see cref="SetPhoneAlarm"/> through <paramref name="later"/>, completing lateness
    /// counts, dismissing the NPC on the third count, and advancing the account schedule.
    /// </summary>
    public void UpdateLateness(DateTime later)
    {
        if (alarmNpc != null)
        {
            alarmNpc.IsLateToWork(later, out bool shouldReset);
            if (shouldReset)
            {
                if (alarmNpc.ResetLateToWork())
                {
                    alarmNpc.SetFired(alarmRouter, alarmRouter.GetNetworkLan());
                }
                Database.Singleton.SyncConfigOS(alarmRouter);
                Computer workstation = alarmRouter.GetComputer(alarmNpc.GetLocalIP());
                Persona account = workstation?.GetPersona(alarmNpc.GetSurname(), true);
                if (account != null)
                {
                    account.npcInfo = alarmNpc;
                    Database.Singleton.SyncConfigOS(workstation);
                }
            }
        }
        if (alarmPersona != null)
        {
            alarmPersona.npcInfo = alarmNpc;
            alarmPersona.NextCita();
        }
    }

    public Reply ForceAdminLogin(ForceAdminLoginRpc request)
    {
        Router router = GetNetwork(request.PublicIp);

        (
            router.admin
            ?? throw new InvalidOperationException("No administrator at " + request.PublicIp)
        ).ForceLogin();
        return new Reply.Applied();
    }

    /// <summary>
    /// Has the network administrator report the newest library as the network's update
    /// (<c>Persona.UpdateReportedNetworks</c>), and replies with the library the game reported, or with no content
    /// when the guild owning the company turned Admin Restore off and the administrator skipped the update.
    /// </summary>
    public Reply ReportLibraryUpdate(ReportLibraryUpdateRpc request)
    {
        Router router = GetNetwork(request.PublicIp);
        Persona admin =
            router.admin
            ?? throw new InvalidOperationException("No administrator at " + request.PublicIp);
        string result = (string)
            GameMethod.PersonaUpdateReportedNetworks.Invoke(
                admin,
                new object[] { request.PublicIp, request.Library }
            );

        if (result == TranslationManager.GetText("ING_UPDATE_LIBS_SKIPPED"))
        {
            return new Reply.LibraryReported(new LibraryReportedReply(""));
        }
        if (result != TranslationManager.GetText("ING_UPDATE_LIBS_OK"))
        {
            throw new InvalidOperationException(result);
        }

        return new Reply.LibraryReported(
            new LibraryReportedReply(
                JsonConvert.SerializeObject(
                    VersionsControl.Singleton.GetLastLibs()[request.Library]
                )
            )
        );
    }

    /// <summary>Creates a new-network administrator for a rented server using the bank website behavior.</summary>
    public Reply HireRentalAdmin(HireRentalAdminRpc request)
    {
        Router router = GetNetwork(request.PublicIp);
        List<ServerMap.TipoRed> kinds = OS.GetListTipoRedComun();
        Random random = new Random(Guid.NewGuid().GetHashCode());
        Router employer = ServerMap.Singleton.SpawnRouter(kinds[random.Next(kinds.Count)]);

        router
            .GetNetworkLan()
            .SetSuperAdminRental(employer.GetNetworkLan().GetRandomNpc(), employer.GetPublicIP());
        Database.Singleton.SyncConfigOS(router);
        return new Reply.Applied();
    }

    private static Router GetNetwork(string publicIp) =>
        ServerMap.Singleton.GetRouter(new IP(publicIp), false)
        ?? throw new InvalidOperationException("No network at " + publicIp);
}
