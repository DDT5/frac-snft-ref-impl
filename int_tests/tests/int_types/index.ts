import { SecretNetworkClient, Wallet, } from "secretjs";
import { AminoWallet } from "secretjs/dist/wallet_amino";
import { FracInitMsg, FracInitResponse, FracHandleMsg, FracHandleResponse, FracQueryMsg, FracQueryResponse, } from "./fractionalizer";
import { FtokenInitMsg, FtokenInitResponse, FtokenHandleMsg, FtokenHandleResponse, FtokenQueryMsg, FtokenQueryResponse } from "./ftoken";
import { u64 } from "./utils";
// export * from "./fractionalizer"
// export * from "./ftoken"

/////////////////////////////////////////////////////////////////////////////////
// Environment and accounts
/////////////////////////////////////////////////////////////////////////////////

export type jsEnv = {
  accounts: Account[];
  uploadedCodes: UploadedCode[];
  contracts: jsContractInfo[];
}

export type Account = {
  address: string;
  mnemonic: string;
  walletAmino: AminoWallet;
  walletProto: Wallet;
  secretjs: SecretNetworkClient;
};

/////////////////////////////////////////////////////////////////////////////////
// Interfaces
/////////////////////////////////////////////////////////////////////////////////

export type UploadedCode = {
  codeId: u64,
  hash: string;
}

export type jsContractInfo = {
  hash: string;
  address: string;
}

// export type Balance = {
//   address: string,
//   amount: string,
// };

/////////////////////////////////////////////////////////////////////////////////
// Combined messages
/////////////////////////////////////////////////////////////////////////////////

export type InitMsg = FracInitMsg | FtokenInitMsg
export type InitResponse = FracInitResponse | FtokenInitResponse
export type HandleMsg = FracHandleMsg | FtokenHandleMsg
export type HandleResponse = FracHandleResponse | FtokenHandleResponse 
export type QueryMsg = FracQueryMsg | FtokenQueryMsg
export type QueryResponse = FracQueryResponse | FtokenQueryResponse  


