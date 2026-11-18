using System;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using HarmonyLib;
using MailConfig;
using ServiceConfig;
using ServicioID = ServiceConfig.ServicioID;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Creates external player accounts for debug access, CTF events, and mail.</summary>
internal sealed class AccountFixture(Player player) : IFixture
{
    /// <summary>Credentials returned by <see cref="CreateDebug"/> and used by network queries to read debug state.</summary>
    public string DebugUser { get; private set; }

    public string DebugPassword { get; private set; }

    /// <summary>CTF event created by <see cref="CreateCtf"/> and exposed through network queries.</summary>
    public string CtfEventName { get; private set; }

    public void Reset()
    {
        DebugUser = null;
        DebugPassword = null;
        CtfEventName = null;
    }

    /// <summary>
    /// Obtains the current player's credentials using the zero-day rumor path
    /// (<c>ZeroDaySystem.GetNpcPassword</c>) and configures test scripts to run as root.
    /// </summary>
    public Reply CreateDebug()
    {
        PlayerServer server = player.Server;
        ZeroDayPlayerInfo zeroDay = Database.Singleton.GetZeroDayRequestPlayer(
            server.GetPlayerID()
        );
        zeroDay.ResetLoginAttempts();
        zeroDay.ResetSoloAttempts();
        Database.Singleton.SyncZeroDayRequestPlayer(server.GetPlayerID(), zeroDay, false, false);
        (string user, string password) = ZeroDaySystem.Singleton.GetNpcPassword(
            server.GetPlayerID()
        );
        DebugUser = user;
        DebugPassword = password;
        player.User = "root";
        return new Reply.DebugReady(new DebugReadyReply(user, password));
    }

    /// <summary>Creates and accepts a CTF event through the same operations as the CTF browser server RPCs.</summary>
    public Reply CreateCtf(SetupCtfRpc request)
    {
        PlayerServer server = player.Server;
        Router template = ServerMap.Singleton.SpawnRouter(
            ServerMap.TipoRed.RentServer,
            NetworkLan.AccessType.NORMAL,
            null,
            false
        );
        server.playerHelper.rentalIPs[template.GetPublicIP()] = true;
        Database.Singleton.SyncRentalsInfo(server.GetPlayerID(), server.playerHelper.rentalIPs);
        const string user = "ctfuser";
        const string password = "oldpass";
        server.missionsHelper.CreateCTFAccountServerRpc(user, password, -1);
        // Creating a new player computer does not reset the real-time account cooldown, so this fixture clears it.
        GameField.CtfCooldown.SetValue(server.missionsHelper, new CTFCooldown());
        server.missionsHelper.CreateCTFServerRpc(
            request.EventName,
            "A parity mission",
            template.GetPublicIP(),
            "Parity creator",
            "The mission briefing",
            -1
        );
        CTF_Event gameEvent = Database.Singleton.GetEvent(request.EventName);
        PlayerComputer computer = server.GetComputer();
        // Execute only the participation part of `AcceptMissionCTFServerRpc`. Its later `ServerMap.SpawnCTF` call
        // requires UI CTF service configuration that the event-query intrinsics do not use.
        if (
            gameEvent == null
            || !gameEvent.AddPlayer(
                server.GetPlayerID(),
                computer.GetMailAddress(),
                computer.GetUserBank().userName
            )
        )
        {
            throw new InvalidOperationException("The game didn't create and accept the CTF event");
        }
        CtfEventName = request.EventName;
        return new Reply.Applied();
    }

    /// <summary>Rates an event as <c>MissionsHelperServer.RateCTFServerRpc</c> does, through <c>CTF_Event.AddScore</c>.</summary>
    public Reply WriteCtfScore(RateCtfRpc request)
    {
        CTF_Event gameEvent = Database.Singleton.GetEvent(request.EventName, "", "", false);
        string output;

        if (gameEvent == null)
        {
            output = "Error: CTF event not found";
        }
        else if (gameEvent.IsLocked())
        {
            output = "This mission is locked and cannot be rated.";
        }
        else
        {
            bool rated = gameEvent.AddScore(
                player.Server.GetPlayerID(),
                (int)request.Quality,
                (int)request.Difficulty,
                out output
            );

            if (rated)
            {
                output = "Mission rated successfully";
            }
        }

        return new Reply.CtfRated(new CtfRatedReply(output));
    }

    /// <summary>Applies mail body and recipient-filter state, or removes an account during a paused script.</summary>
    public Reply ConfigureMail(ConfigureMailRpc request)
    {
        switch (request.Fixture)
        {
            case "long_body":
                UserMail account = Database.Singleton.GetMailAccount(request.Address, out _);
                account.RecibirMail("Long body", new string('x', 200), "fixture@example");
                Database.Singleton.UserMailUpdate(account);
                break;
            case "delete_account":
                object db = Traverse
                    .Create(Database.Singleton)
                    .Method("Config", "DELETE FROM MailAccounts WHERE User=@address")
                    .GetValue();
                db.GetType()
                    .GetMethod("AddParameter", new[] { typeof(string), typeof(string) })
                    .Invoke(db, new object[] { "@address", request.Address });
                db.GetType().GetMethod("ExecuteNonQuery", Type.EmptyTypes).Invoke(db, null);
                ((IDisposable)db).Dispose();
                break;
            case "blacklist":
            case "block_players":
            case "block_npcs":
            case "npc_sender":
                UserMail recipient = Database.Singleton.GetMailAccount(request.Address, out _);
                recipient.playerPcID = "fixture-recipient";
                recipient.spamFilter =
                    request.Fixture == "block_players" ? 0
                    : request.Fixture == "block_npcs" || request.Fixture == "npc_sender" ? 1
                    : 2;
                if (request.Fixture == "blacklist" && !recipient.blacklist.Contains(request.Sender))
                {
                    recipient.blacklist.Add(request.Sender);
                }
                Database.Singleton.UserMailUpdate(recipient);
                if (request.Fixture == "npc_sender")
                {
                    UserMail sender = Database.Singleton.GetMailAccount(request.Sender, out _);
                    sender.playerPcID = "";
                    Database.Singleton.UserMailUpdate(sender);
                }
                break;
            default:
                throw new InvalidOperationException("Unknown mail fixture: " + request.Fixture);
        }

        return new Reply.Applied();
    }

    /// <summary>Deletes an event through the game's own CTF database operation.</summary>
    public Reply DeleteCtf(DeleteCtfRpc request)
    {
        CTF_Event gameEvent = Database.Singleton.GetEvent(request.EventName);
        if (gameEvent == null)
        {
            throw new InvalidOperationException("The CTF event does not exist");
        }
        Database.Singleton.DeleteEvent(gameEvent);
        return new Reply.Applied();
    }

    /// <summary>
    /// Registers the player's <c>user@domain</c> mail account and a recipient account on the network SMTP server.
    /// </summary>
    public Reply CreateMail(SetupMailRpc request)
    {
        PlayerComputer computer = player.Computer;
        Router router = computer.GetMainRouter();
        LanPcInfo server = router.GetServidorLan(ServicioID.smtp, out Servicio service);
        if (server == null)
        {
            throw new InvalidOperationException("The current network has no SMTP server");
        }
        string address = request.User + "@" + router.GetDomain();
        string result = Database.Singleton.WaitFileMailRegister(
            computer,
            request.User,
            request.Password,
            PasswordsEncrypter.Encrypt(request.Password),
            address
        );
        if (!result.StartsWith("Registration OK", StringComparison.Ordinal))
        {
            throw new InvalidOperationException(result);
        }
        string recipient = "recipient@" + router.GetDomain();
        if (!Database.Singleton.ExistMailAccount(recipient))
        {
            Database.Singleton.UserMailUpdate(
                new UserMail(
                    recipient,
                    "recipientpass",
                    PasswordsEncrypter.Encrypt("recipientpass")
                )
            );
        }
        return new Reply.Applied();
    }
}
