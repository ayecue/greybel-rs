using System;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using HarmonyLib;
using Newtonsoft.Json;
using OS = Util.OS;

namespace GreyHackTestSupport.Plugin.Bridge.Fixtures;

/// <summary>Sets up bank accounts, wallets, coins, and markets for the player and counterparties.</summary>
internal sealed class EconomyFixture(Player player)
{
    /// <summary>
    /// Creates a player bank account and the coin <c>request.CoinName</c>. Two NPC wallets, a seller and a buyer,
    /// receive standing offers for the coin, whose first mining round is immediately due.
    /// </summary>
    public Reply CreateBlockchain(SetupBlockchainRpc request)
    {
        PlayerServer server = player.Server;
        PlayerComputer computer = server.GetComputer();
        PlayerConfigOS config = computer.GetPlayerConfigOS();

        while (
            Database.Singleton.GetCoinCredentials(
                server.playerID,
                out string oldUser,
                out string oldPassword,
                out string oldName
            )
        )
        {
            string removeError = BlockchainSystem.RemoveCurrency(oldName, oldUser, oldPassword);
            if (removeError.Length > 0)
            {
                throw new InvalidOperationException(removeError);
            }
        }
        config.SetWallet("");
        config.RemoveCoin();

        Router router = computer.GetMainRouter();
        string domain = router.GetDomain() ?? "";
        string bankNumber = OS.GeneraBankAccount(domain, new Random());
        BankAccount bank = new BankAccount(
            bankNumber,
            "test",
            domain,
            computer.GetPublicIP(),
            isPlayer: true
        );
        bank.Ingreso("Unknown", "Initial funds", 100000);
        Database.Singleton.WaitFileBankRegister(
            computer,
            bank,
            bankNumber,
            "test",
            Computer.Md5Sum("test")
        );
        BlockchainSystem.Singleton.AddCurrency(
            request.CoinName,
            server.playerID,
            player.User,
            "matrix1",
            "45.45.45.45",
            computer
        );
        bank.Ingreso("Unknown", "Coin creation", -150);
        Database.Singleton.SyncBankAccount(bank);

        string counterBankNumber =
            FindNpcBank(router, router.GetNetworkLan().routerDevice, bankNumber) ?? bankNumber;
        BankAccount counterBank = Database.Singleton.GetBankAccount(counterBankNumber, out _);
        counterBank.Ingreso("Unknown", "Initial funds", 100000);
        Database.Singleton.SyncBankAccount(counterBank);

        string counterUser = "c" + request.WalletUser;
        BlockchainSystem.Wallet counterWallet = new BlockchainSystem.Wallet(
            "fixture-" + Guid.NewGuid().ToString("N"),
            counterUser,
            "counterpass",
            counterBankNumber
        );
        Database.Singleton.AddWallet(counterWallet);
        string pin = counterWallet.GetPin();
        string error = BlockchainSystem.AddSubWallet(
            request.CoinName,
            counterUser,
            pin,
            "seller",
            "sellerpass"
        );
        if (error.Length > 0)
        {
            throw new InvalidOperationException(error);
        }
        counterWallet = Database.Singleton.GetWallet(counterUser);
        counterWallet.AddBalance(request.CoinName, 100, "seller");
        BlockchainSystem.Coin coin = Database.Singleton.GetCoin(request.CoinName);
        coin.mined += 100;
        Database.Singleton.SyncWallet(counterWallet);
        Database.Singleton.SyncCoin(coin);
        BlockchainSystem.Wallet walletRow = Database.Singleton.GetWallet(counterUser);
        if (walletRow.GetSubWallet(request.CoinName, "seller") == null)
        {
            throw new InvalidOperationException(JsonConvert.SerializeObject(walletRow));
        }

        string buyerUser = "b" + request.WalletUser;
        BlockchainSystem.Wallet buyerWallet = new BlockchainSystem.Wallet(
            "fixture-" + Guid.NewGuid().ToString("N"),
            buyerUser,
            "buyerpass",
            counterBankNumber
        );
        Database.Singleton.AddWallet(buyerWallet);
        error = BlockchainSystem.AddSubWallet(
            request.CoinName,
            buyerUser,
            buyerWallet.GetPin(),
            "buyer",
            "buyerpass"
        );
        if (error.Length > 0)
        {
            throw new InvalidOperationException(error);
        }
        buyerWallet = Database.Singleton.GetWallet(buyerUser);
        buyerWallet.AddBalance(request.CoinName, 20, "buyer");
        coin = Database.Singleton.GetCoin(request.CoinName);
        coin.mined += 20;
        Database.Singleton.SyncWallet(buyerWallet);
        Database.Singleton.SyncCoin(coin);
        coin.sellOffers.Add(
            counterUser,
            new BlockchainSystem.HelperOffer(5, 5, counterBankNumber, "seller")
        );
        coin.buyOffers.Add(
            buyerUser,
            new BlockchainSystem.HelperOffer(5, 20, counterBankNumber, "buyer")
        );
        coin.mined = 120;
        GameField.CoinNextCycle.SetValue(coin, ClockServer.Singleton.GetDateTime().AddHours(-1));
        Database.Singleton.SyncCoin(coin);
        return new Reply.BlockchainReady(new BlockchainReadyReply(bankNumber));
    }

    /// <summary>Applies an exact stale-account, market, or mining boundary state.</summary>
    public Reply Configure(ConfigureEconomyRpc request)
    {
        switch (request.Fixture)
        {
            case "delete_wallet":
                foreach (string coinName in Database.Singleton.GetCoinNames())
                {
                    BlockchainSystem.Coin coin = Database.Singleton.GetCoin(coinName);
                    coin.buyOffers.Remove(request.Wallet);
                    coin.sellOffers.Remove(request.Wallet);
                    Database.Singleton.SyncCoin(coin);
                }
                object db = Traverse
                    .Create(Database.Singleton)
                    .Method("Config", "DELETE FROM Wallets WHERE WalletID = @walletID")
                    .GetValue();
                Traverse.Create(db).Method("AddParameter", "@walletID", request.Wallet).GetValue();
                Traverse.Create(db).Method("ExecuteNonQuery").GetValue();
                (db as IDisposable)?.Dispose();
                object clear = Traverse
                    .Create(Database.Singleton)
                    .Method("Config", "UPDATE Players SET WalletID = '' WHERE PlayerID = @playerID")
                    .GetValue();
                Traverse
                    .Create(clear)
                    .Method("AddParameter", "@playerID", player.Server.playerID)
                    .GetValue();
                Traverse.Create(clear).Method("ExecuteNonQuery").GetValue();
                (clear as IDisposable)?.Dispose();
                player.Computer.GetPlayerConfigOS().SetWallet("");
                Database.Singleton.SyncConfigOS(player.Computer);
                break;
            case "delete_coin":
                Database.Singleton.DeleteCoin(request.Coin);
                break;
            case "delete_subwallet":
                BlockchainSystem.Wallet wallet = Database.Singleton.GetWallet(request.Wallet);
                wallet.RemoveSubWallet(request.Coin, request.Subwallet, out _);
                Database.Singleton.SyncWallet(wallet);
                break;
            case "unbanked_wallet":
                BlockchainSystem.Wallet unbanked = new BlockchainSystem.Wallet(
                    "fixture-" + Guid.NewGuid().ToString("N"),
                    request.Wallet,
                    "matrix1",
                    "missing-bank"
                );
                Database.Singleton.AddWallet(unbanked);
                string error = BlockchainSystem.AddSubWallet(
                    request.Coin,
                    request.Wallet,
                    unbanked.GetPin(),
                    request.Subwallet,
                    "pass1"
                );
                if (error.Length > 0)
                {
                    throw new InvalidOperationException(error);
                }
                break;
            case "fully_mined":
                BlockchainSystem.Coin mined = Database.Singleton.GetCoin(request.Coin);
                mined.mined = BlockchainSystem.MAX_COINS;
                Database.Singleton.SyncCoin(mined);
                break;
            case "damaged_gpu":
                Hardware hardware = player.Computer.GetHardware();
                hardware.gpu = new Hardware.GPU("fixture", 1, new Random(1));
                hardware.gpu.health = 4;
                Database.Singleton.SyncHardwareDB(player.Computer.GetID(), hardware);
                break;
            default:
                throw new InvalidOperationException("Unknown economy fixture: " + request.Fixture);
        }
        return new Reply.Applied();
    }

    /// <summary>Creates a player wallet with no password-reset cooldown.</summary>
    public Reply CreateWallet(SetupWalletRpc request)
    {
        PlayerServer server = player.Server;
        PlayerComputer computer = server.GetComputer();
        HelperAccount bank = computer.GetUserBank();
        string error = BlockchainSystem.AddWallet(
            computer,
            server.playerID,
            request.User,
            request.Password,
            bank.userName
        );
        if (error.Length > 0)
        {
            throw new InvalidOperationException(error);
        }
        BlockchainSystem.Wallet wallet = Database.Singleton.GetWallet(request.User);
        GameField.WalletLastPasswordReset.SetValue(
            wallet,
            ClockServer.Singleton.GetDateTime().AddDays(-1)
        );
        Database.Singleton.SyncWallet(wallet);
        return new Reply.Applied();
    }

    /// <summary>Creates a counterparty-owned coin.</summary>
    public Reply CreateForeignCoin(SetupForeignCoinRpc request)
    {
        BlockchainSystem.Coin coin = new BlockchainSystem.Coin(
            "fixture-" + Guid.NewGuid().ToString("N"),
            request.CoinName,
            request.User,
            request.Password,
            "45.45.45.45"
        );
        Database.Singleton.AddCoin(coin);
        Database.Singleton.SyncCoin(coin);
        Database.Singleton.AddPassword(
            PasswordsEncrypter.Encrypt(request.Password),
            request.Password
        );
        return new Reply.Applied();
    }

    /// <summary>Finds an NPC bank account number under <paramref name="device"/>, excluding <paramref name="exclude"/>.</summary>
    private static string FindNpcBank(Router main, NetworkLan.Device device, string exclude)
    {
        foreach (LanPcInfo info in device.GetLanPcInfos())
        {
            Computer computer = main.GetComputer(info.localIp);
            foreach (Computer.User user in computer.GetUsers())
            {
                FileSystem.Archivo file = computer
                    .GetFileSystem()
                    .GetArchivo("/home/" + user.nombreUsuario + "/Config/Bank.txt");
                string number = file?.GetContenido()?.Split(':')[0];
                if (
                    !string.IsNullOrEmpty(number)
                    && number != exclude
                    && Database.Singleton.GetBankAccount(number, out _) != null
                )
                {
                    return number;
                }
            }
        }
        foreach (NetworkLan.Device child in device.GetDevices())
        {
            string number = FindNpcBank(main, child, exclude);
            if (number != null)
            {
                return number;
            }
        }
        return null;
    }
}
