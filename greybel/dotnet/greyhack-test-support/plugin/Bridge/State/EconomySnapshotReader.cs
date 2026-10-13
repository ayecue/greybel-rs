using System.Collections.Generic;
using System.Linq;
using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.State;

/// <summary>Reads game economy state for banks, coins, wallets, and offers.</summary>
internal static class EconomySnapshotReader
{
    /// <summary>
    /// Gets the player's bank account and wallet plus every coin, its offers, and the wallets and bank accounts
    /// associated with its subwallets.
    /// </summary>
    public static EconomySnapshot Read(PlayerComputer player)
    {
        Dictionary<string, BankSnapshot> banks = new Dictionary<string, BankSnapshot>();
        Dictionary<string, WalletSnapshot> wallets = new Dictionary<string, WalletSnapshot>();
        List<CoinSnapshot> coins = new List<CoinSnapshot>();
        List<OfferSnapshot> offers = new List<OfferSnapshot>();
        HelperAccount playerBank = player.GetUserBank();
        if (playerBank != null)
        {
            ReadBank(playerBank.userName, banks);
        }
        string playerWallet = player.GetPlayerConfigOS().GetWalletID();
        if (!string.IsNullOrEmpty(playerWallet) && !wallets.ContainsKey(playerWallet))
        {
            BlockchainSystem.Wallet wallet = Database.Singleton.GetWallet(playerWallet);
            wallets.Add(playerWallet, ReadWallet(wallet));
            ReadBank(wallet.GetUserBank(), banks);
        }
        foreach (string name in Database.Singleton.GetCoinNames())
        {
            BlockchainSystem.Coin coin = Database.Singleton.GetCoin(name);
            coins.Add(ReadCoin(coin));
            foreach (string walletId in coin.subWallets.Values)
            {
                if (!wallets.ContainsKey(walletId))
                {
                    BlockchainSystem.Wallet wallet = Database.Singleton.GetWallet(walletId);
                    wallets.Add(walletId, ReadWallet(wallet));
                    ReadBank(wallet.GetUserBank(), banks);
                }
            }
            foreach (KeyValuePair<string, BlockchainSystem.HelperOffer> offer in coin.buyOffers)
            {
                offers.Add(ReadOffer(name, offer, "Buy"));
            }
            foreach (KeyValuePair<string, BlockchainSystem.HelperOffer> offer in coin.sellOffers)
            {
                offers.Add(ReadOffer(name, offer, "Sell"));
            }
        }

        return new EconomySnapshot(
            playerBank?.userName,
            banks.Values.ToList(),
            coins,
            wallets.Values.ToList(),
            offers
        );
    }

    private static void ReadBank(string number, Dictionary<string, BankSnapshot> banks)
    {
        if (banks.ContainsKey(number))
        {
            return;
        }
        BankAccount bank = Database.Singleton.GetBankAccount(number, out _);
        if (bank != null)
        {
            banks.Add(number, new BankSnapshot(number, bank.password, bank.GetMoney()));
        }
    }

    private static CoinSnapshot ReadCoin(BlockchainSystem.Coin coin)
    {
        return new CoinSnapshot(
            coin.coinName,
            coin.GetUser(),
            coin.GetPassword(),
            coin.GetAddress(),
            coin.coinRate,
            coin.reward,
            coin.mined,
            100000000.0,
            coin.moneyValue,
            coin.record.Select(price => new CoinPriceSnapshot(
                    price.price,
                    price.date.GetFechaCompleta()
                ))
                .ToList()
        );
    }

    private static WalletSnapshot ReadWallet(BlockchainSystem.Wallet wallet)
    {
        List<SubWalletSnapshot> subwallets = new List<SubWalletSnapshot>();
        foreach (
            KeyValuePair<
                string,
                Dictionary<string, BlockchainSystem.SubWallet>
            > coin in wallet.GetBalance()
        )
        {
            foreach (BlockchainSystem.SubWallet subwallet in coin.Value.Values)
            {
                subwallets.Add(
                    new SubWalletSnapshot(
                        coin.Key,
                        subwallet.GetUser(),
                        subwallet.GetPassword(),
                        subwallet.GetBalance(),
                        subwallet.GetInfo() ?? ""
                    )
                );
            }
        }
        return new WalletSnapshot(
            wallet.walletID,
            wallet.GetPassword(),
            wallet.GetPin(),
            wallet.GetUserBank(),
            subwallets
        );
    }

    private static OfferSnapshot ReadOffer(
        string coin,
        KeyValuePair<string, BlockchainSystem.HelperOffer> offer,
        string side
    )
    {
        return new OfferSnapshot(
            coin,
            offer.Key,
            side,
            offer.Value.propPrice,
            offer.Value.coinsAmount,
            offer.Value.subwalletID
        );
    }
}

/// <summary>Economy data populated by <see cref="EconomySnapshotReader"/> in a <see cref="NetworkSnapshot"/>.</summary>
internal sealed class EconomySnapshot(
    string playerBank,
    List<BankSnapshot> banks,
    List<CoinSnapshot> coins,
    List<WalletSnapshot> wallets,
    List<OfferSnapshot> offers
)
{
    public string PlayerBank { get; } = playerBank;

    public List<BankSnapshot> Banks { get; } = banks;

    public List<CoinSnapshot> Coins { get; } = coins;

    public List<WalletSnapshot> Wallets { get; } = wallets;

    public List<OfferSnapshot> Offers { get; } = offers;
}
