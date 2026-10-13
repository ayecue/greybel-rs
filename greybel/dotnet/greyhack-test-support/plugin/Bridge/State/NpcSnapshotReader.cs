using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using HarmonyLib;

namespace GreyHackTestSupport.Plugin.Bridge.State;

/// <summary>Reads network NPCs, administrator state, and police traffic-camera state.</summary>
internal static class NpcSnapshotReader
{
    public static List<NpcSnapshot> ReadNpcs(Router router, NetworkLan lan)
    {
        DateTime now = ClockServer.Singleton.GetDateTime();
        return lan.GetNpcInfos()
            .Values.Select(npc =>
            {
                Activity activity = npc.schedule.GetCurrentActivity(now.DayOfWeek, now.TimeOfDay);
                AlarmConfig alarm = npc.GetAlarmConfig();
                TimeSpan? wakeUpTime = Traverse.Create(npc).Field<TimeSpan?>("wakeUpTime").Value;
                int dayAlarm = Traverse.Create(npc).Field<int>("dayAlarm").Value;
                return new NpcSnapshot(
                    npc.GetLocalIP(),
                    npc.GetSurname(),
                    npc.GetNombreCompleto(),
                    (byte)npc.GetJobRole(),
                    npc.GetLicensePlate() ?? "",
                    npc.schedule.dayTime,
                    (ulong)npc.schedule.GetActivityTimeStart(Activity.Trabajando).TotalSeconds,
                    (ulong)
                        (
                            npc.schedule.GetActivityTimeStart(Activity.ViajeCasa)
                            - TimeSpan.FromHours(4)
                        ).TotalSeconds,
                    (ulong)npc.schedule.GetTimeWalkToAppliance().TotalSeconds,
                    activity == Activity.ViajeTrabajo || activity == Activity.ViajeCasa,
                    npc.IsDead(),
                    npc.IsForcedLogin() || activity == Activity.Trabajando,
                    npc.IsForcedLogin()
                        ? (ulong?)
                            ((DateTime)GameField.NpcForcedLoginEnd.GetValue(npc) - now).TotalSeconds
                        : null,
                    (byte)alarm.hour,
                    (byte)alarm.minute,
                    alarm.isAlarmModified,
                    alarm.isAlarmDeleted,
                    wakeUpTime.HasValue ? (ulong?)wakeUpTime.Value.TotalSeconds : null,
                    (uint)dayAlarm,
                    (byte)npc.GetLateWorkStrikes(),
                    npc.IsFired(),
                    (byte)npc.GetWorkPerformance(),
                    (byte)npc.GetCompanyAlign(),
                    npc.GetDayZeroContact() ?? "",
                    AccountSnapshotReader.ReadNpcMail(router, npc)
                );
            })
            .ToList();
    }

    public static AdminSnapshot ReadAdmin(NpcInfo admin)
    {
        if (admin == null)
        {
            return null;
        }

        return new AdminSnapshot(
            (byte)(int)GameField.NpcIntelligence.GetValue(admin),
            (byte)(int)GameField.NpcPcExperience.GetValue(admin),
            (byte)admin.GetWorkPerformance(),
            admin.IsDeadOrFired(),
            admin.GetSurname()
        );
    }

    public static TrafficSnapshot ReadTraffic(Router router, NetworkLan lan, string accidentAt)
    {
        if (router.GetTipoRed() != ServerMap.TipoRed.Comisaria)
        {
            return null;
        }
        // Read the stored camera-login account (NetworkLan.SelectCredentialsTrafficNpc) without generating one.
        // Some police networks have none, causing NetworkLan.GetNpcInfoTrafficCrecentials and locate_vehicle to fail.
        return new TrafficSnapshot(
            Traverse.Create(lan).Field<string>("trafficCredentials").Value ?? "",
            router.GetConfigOS().IsOverrideSettingsEnabled(),
            router.GetConfigOS().IsOverrideUsed(),
            accidentAt
        );
    }
}
