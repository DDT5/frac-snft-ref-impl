export type HumanAddr = string;
export type Uint128 = string;
export type Binary = string;
export type u8 = number;
export type u32 = number;
export type u64 = number;

export interface UndrNftInfo {
  token_id: string,
  nft_contr: ContractInfo,
}

/** Matches Rust contract interface */
export type ContractInfo = {
  code_hash: string;
  address: string;
}