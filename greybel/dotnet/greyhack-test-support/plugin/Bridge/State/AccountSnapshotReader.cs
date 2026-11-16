using System;
using System.Linq;
using GreyHackTestSupport.Protocol;
using MailConfig;
using MiniscriptChallengeGenerator;
using Newtonsoft.Json;

namespace GreyHackTestSupport.Plugin.Bridge.State;

/// <summary>Reads external player account state for mail, CTF events, and debug access.</summary>
internal static class AccountSnapshotReader
{
    public static MailAccountSnapshot ReadMail(HelperAccount account)
    {
        return account == null || string.IsNullOrEmpty(account.userName)
            ? null
            : ReadMailAddress(account.userName, account.password);
    }

    public static MailAccountSnapshot ReadMailAddress(string address, string password)
    {
        UserMail mail = Database.Singleton.GetMailAccount(address, out _);
        if (mail == null)
        {
            return null;
        }
        return ReadMailAccount(mail, password);
    }

    /// <summary>
    /// Reads an NPC's mail account as stored, without generating its mails
    /// (<c>Database.GetMailAccount</c> with <c>generateMailContents: false</c>).
    /// </summary>
    public static MailAccountSnapshot ReadNpcMail(Router router, NpcInfo npc)
    {
        UserMail mail = Database.Singleton.GetMailAccount(
            npc.GetMailAddress(),
            out _,
            router,
            generateMailContents: false
        );
        return mail == null ? null : ReadMailAccount(mail, npc.GetEmailPassword());
    }

    private static MailAccountSnapshot ReadMailAccount(UserMail mail, string password)
    {
        return new MailAccountSnapshot(
            mail.address,
            password,
            mail.emails.Select(email => new MailSnapshot(
                    email.GetID(),
                    email.otherMail,
                    email.messages[0].titulo,
                    email.messages[0].mensaje,
                    email.messages.Skip(1).Select(message => message.mensaje).ToList()
                ))
                .ToList(),
            mail.npcContentsGenerated,
            mail.playerPcID ?? "",
            (byte)mail.spamFilter,
            mail.blacklist.ToList()
        );
    }

    public static CtfSnapshot ReadCtf(Computer player, string eventName)
    {
        if (string.IsNullOrEmpty(eventName))
        {
            return null;
        }
        CTF_Event gameEvent = Database.Singleton.GetEvent(eventName);
        CTF_Event.CTF_Player participant;
        bool completed =
            gameEvent.players.TryGetValue(((PlayerComputer)player).GetPlayerID(), out participant)
            && participant.IsCompleted();
        return new CtfSnapshot(
            gameEvent.GetEventName(),
            gameEvent.GetUser(),
            gameEvent.GetPassword(),
            gameEvent.GetDescription(),
            gameEvent.GetPublicAddress(),
            gameEvent.GetCreatorName(),
            gameEvent.GetMailContent(),
            completed
        );
    }

    /// <summary>Gets challenge and patch state derived from the player library by debug intrinsics.</summary>
    public static DebugSnapshot ReadDebug(Computer player, string user, string password)
    {
        if (string.IsNullOrEmpty(user))
        {
            return null;
        }
        const string path = "/lib/metaxploit.so";
        FileSystem.Archivo file = player.GetFileSystem().GetArchivo(path);
        Library library = JsonConvert.DeserializeObject<Library>(file.GetContenido());
        ChallengeGenerator generator = new ChallengeGenerator(library);
        Challenge challenge = generator.GenerateChallenge();
        library.GenerateZeroDayVulnerability(
            new Random(generator.seed),
            out Library.MemoryZone memoryZone
        );
        string autoPatch = library.GetAutoPatchDate().ToString("dd/MM/yyyy");
        return new DebugSnapshot(
            user,
            password,
            path,
            generator.seed,
            autoPatch == "01/01/0001" ? null : autoPatch,
            challenge.Description,
            challenge.AdditionalDescriptions ?? "",
            challenge.CodeWithErrors,
            challenge.CorrectCode,
            challenge.ErrorLineNumbers.Select(line => (long)line).ToList(),
            library.IsPatchApplied(generator.seed),
            memoryZone.address
        );
    }
}
