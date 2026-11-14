using System;
using System.Collections.Generic;
using System.Linq;
using GreyHackMessageHook;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>
/// Registers the player's rented networks and runs the monthly bank payments (<c>BankSubs</c>), and hires or removes
/// a rented network's administrator through the bank's page. Each player setup removes what a test registered.
/// </summary>
internal sealed class BankFixture(Player player) : IFixture
{
    /// <summary>Payment kinds in the order the snapshot lists them and payments run.</summary>
    private static readonly BankSubs.PaymentID[] PaymentOrder =
    [
        BankSubs.PaymentID.BANK,
        BankSubs.PaymentID.RENTED,
        BankSubs.PaymentID.ADMIN,
    ];

    /// <summary>Addresses <see cref="CreateRental"/> put in the player's rental list.</summary>
    private readonly List<string> rentalAddresses = new List<string>();

    public void Reset()
    {
        PlayerServer server = PlayerServerEx.Current;

        if (server == null)
        {
            rentalAddresses.Clear();
            return;
        }

        foreach (BankSubs.PaymentID kind in PaymentOrder)
        {
            BankSubs.Singleton.RemovePayment(server.GetPlayerID(), kind);
        }
        foreach (string publicIp in rentalAddresses)
        {
            server.playerHelper.rentalIPs.Remove(publicIp);
        }
        Database.Singleton.SyncRentalsInfo(server.GetPlayerID(), server.playerHelper.rentalIPs);
        rentalAddresses.Clear();
    }

    /// <summary>
    /// Makes the network at <c>request.PublicIp</c> one the player rents, as <c>BankHelperServer.RentServer</c>
    /// does: owned by the player, in the player's rental list, and charged by the rental payment.
    /// </summary>
    public Reply CreateRental(RentNetworkRpc request)
    {
        PlayerServer server = player.Server;
        Router router =
            ServerMap.Singleton.GetRouter(new IP(request.PublicIp), false)
            ?? throw new InvalidOperationException("No network at " + request.PublicIp);

        router.GetNetworkLan().playerOwnerID = server.GetPlayerID();
        Database.Singleton.SyncConfigOS(router);
        server.playerHelper.rentalIPs[router.GetPublicIP()] = false;
        Database.Singleton.SyncRentalsInfo(server.GetPlayerID(), server.playerHelper.rentalIPs);
        rentalAddresses.Add(router.GetPublicIP());
        BankSubs.Singleton.AddPayment(
            server.GetPlayerID(),
            BankSubs.PaymentID.RENTED,
            (int)request.Amount,
            server.GetComputer().GetUserBank().userName,
            router.GetPublicIP()
        );
        return new Reply.Applied();
    }

    /// <summary>Hires or removes the administrator through the bank's page RPCs, which report only to the client.</summary>
    public Reply WriteRentalAdmin(RentalAdminServiceRpc request)
    {
        PlayerServer server = player.Server;

        if (request.Hire)
        {
            server.bankHelper.HireAdminRentalServerRpc(request.PublicIp, -1);
        }
        else
        {
            server.bankHelper.FireAdminRentalServerRpc(request.PublicIp, -1);
        }
        return new Reply.Applied();
    }

    /// <summary>Runs the player's payments in the order the snapshot lists them.</summary>
    public Reply RunPayments()
    {
        string playerId = player.Server.GetPlayerID();

        foreach (BankSubs.PaymentID kind in PaymentOrder)
        {
            BankSubs.Payment payment = BankSubs.Singleton.GetPayment(playerId, kind);

            if (payment != null)
            {
                GameField.PaymentRunPayment.Invoke(payment, null);
            }
        }
        return new Reply.Applied();
    }

    /// <summary>Runs one of the player's payments, as the game does when its own date comes.</summary>
    public Reply RunPayment(RunPaymentRpc request)
    {
        BankSubs.PaymentID kind = (BankSubs.PaymentID)
            Enum.Parse(typeof(BankSubs.PaymentID), request.Kind);
        BankSubs.Payment payment =
            BankSubs.Singleton.GetPayment(player.Server.GetPlayerID(), kind)
            ?? throw new InvalidOperationException(
                "The player has no " + request.Kind + " payment"
            );

        GameField.PaymentRunPayment.Invoke(payment, null);
        return new Reply.Applied();
    }

    /// <summary>Sets the money of the player's bank account.</summary>
    public Reply SetMoney(SetPlayerMoneyRpc request)
    {
        PlayerComputer computer = player.Server.GetComputer();
        BankAccount bank = Database.Singleton.GetBankAccount(
            computer.GetUserBank().userName,
            out string _
        );

        bank.SetMoney((float)request.Money);
        Database.Singleton.SyncBankAccount(bank);
        return new Reply.Applied();
    }

    /// <summary>Reads the player's payments in the order <see cref="RunPayments"/> runs them.</summary>
    public static List<SubscriptionSnapshot> Read(string playerId)
    {
        List<SubscriptionSnapshot> subscriptions = new List<SubscriptionSnapshot>();

        foreach (BankSubs.PaymentID kind in PaymentOrder)
        {
            BankSubs.Payment payment = BankSubs.Singleton.GetPayment(playerId, kind);

            if (payment == null)
            {
                continue;
            }

            // The account and addresses are private members the game serializes.
            JObject fields = JObject.Parse(JsonConvert.SerializeObject(payment));

            subscriptions.Add(
                new SubscriptionSnapshot(
                    kind.ToString(),
                    (string)fields["bankAccount"],
                    payment.nextMoney,
                    payment.nextDate.GetFechaCompleta(),
                    fields["publicIps"].Select(ip => (string)ip).ToList()
                )
            );
        }
        return subscriptions;
    }
}
