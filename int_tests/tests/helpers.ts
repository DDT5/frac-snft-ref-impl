import { Wallet, SecretNetworkClient, Tx } from "secretjs";
import { AminoWallet } from "secretjs/dist/wallet_amino";
import { Account } from "./int_types";


/////////////////////////////////////////////////////////////////////////////////
// Private functions
/////////////////////////////////////////////////////////////////////////////////

async function genNewAccounts(
  grpcWebUrl: string,
  chainId: string,
  numNewAcc: number,
): Promise<Account[]> {
  // Generate non-genesis accounts
  let accounts: Account[] = [];

  for (let i = 0; i <= numNewAcc; i++) {
    const wallet = new AminoWallet();
    const [{ address }] = await wallet.getAccounts();
    const walletProto = new Wallet(wallet.mnemonic);

    accounts[i] = {
      address: address,
      mnemonic: wallet.mnemonic,
      walletAmino: wallet,
      walletProto: walletProto,
      secretjs: await SecretNetworkClient.create({
        grpcWebUrl,
        chainId,
        wallet: wallet,
        walletAddress: address,
      }),
    };

    console.log(`Initialized wallet with address: ${address}`);
  }

  return accounts;
}

/** funding from genesis account `a` */
async function fundFromGenesisAcc(
  accounts: Account[],
  grpcWebUrl: string,
  chainId: string,
) {
  // Initialize genesis accounts
  const mnemonics = [
    "grant rice replace explain federal release fix clever romance raise often wild taxi quarter soccer fiber love must tape steak together observe swap guitar",
    "jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow",
    "chair love bleak wonder skirt permit say assist aunt credit roast size obtain minute throw sand usual age smart exact enough room shadow charge",
    "word twist toast cloth movie predict advance crumble escape whale sail such angry muffin balcony keen move employ cook valve hurt glimpse breeze brick",
  ];

  let genAccounts: Account[] = [];
  for (let i = 0; i < mnemonics.length; i++) {
    const mnemonic = mnemonics[i];
    const walletAmino = new AminoWallet(mnemonic);
    genAccounts[i] = {
      address: walletAmino.address,
      mnemonic: mnemonic,
      walletAmino,
      walletProto: new Wallet(mnemonic),
      secretjs: await SecretNetworkClient.create({
        grpcWebUrl,
        wallet: walletAmino,
        walletAddress: walletAmino.address,
        chainId,
      }),
    };
  }

  // Send 100k SCRT from account 0 to each of the new accounts

  const { secretjs } = genAccounts[0];

  let tx: Tx;
  try {
    tx = await secretjs.tx.bank.multiSend(
      {
        inputs: [
          {
            address: genAccounts[0].address,
            coins: [{ denom: "uscrt", amount: String(100_000 * 1e6 * accounts.length) }],
          },
        ],
        outputs: accounts.map(({ address }) => ({
          address,
          coins: [{ denom: "uscrt", amount: String(100_000 * 1e6) }],
        })),
      },
      {
        broadcastCheckIntervalMs: 100,
        gasLimit: 200_000,
      },
    );
  } catch (e) {
    if (e instanceof Error) {
      throw new Error(`Failed to multisend: ${e.stack}`);
    } else {
      throw new Error(`Failed to multisend.`);
    }
  }

  if (tx.code !== 0) {
    console.error(`failed to multisend coins`);
    throw new Error("Failed to multisend coins to initial accounts");
  }
}


/////////////////////////////////////////////////////////////////////////////////
// Exported functions
/////////////////////////////////////////////////////////////////////////////////

/** create new accounts */ 
export const initClient = async () => {
  const grpcWebUrl = "http://localhost:9091"; 
  const chainId = "secretdev-1";
  const numNewAcc = 3;
  let accounts: Account[] = await genNewAccounts(grpcWebUrl, chainId, numNewAcc);
  await fundFromGenesisAcc(accounts, grpcWebUrl, chainId);

  // returns only new accounts
  return accounts;
};

export async function getScrtBalance(user: Account, addr?: string): Promise<string> {
  let address;
  if (typeof addr === "string") {
    address = addr
  } else {
    address = user.secretjs.address
  }

  let balanceResponse = await user.secretjs.query.bank.balance({
    address,
    denom: "uscrt",
  });
  return balanceResponse.balance!.amount;
}