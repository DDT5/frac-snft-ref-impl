import { ContractInfo, HumanAddr, Uint128, Binary, u8, u32, u64, UndrNftInfo} from "./utils";

/////////////////////////////////////////////////////////////////////////////////
// Instantiation messages
/////////////////////////////////////////////////////////////////////////////////

type UploadedFtkn = {
    code_id: u64,
    code_hash: string,
  }
  
export type FracInitMsg = {  
    uploaded_ftoken: UploadedFtkn
}

//todo
export type FracInitResponse = {  }
  
/////////////////////////////////////////////////////////////////////////////////
// Execute messages
/////////////////////////////////////////////////////////////////////////////////

interface BatchReceiveNft {
    sender: HumanAddr,
    from: HumanAddr,
    token_ids: string[],
    msg?: Binary,
}

interface TransferNft {
    nft_contr_addr: HumanAddr,
    nft_contr_hash: string,
    recipient: HumanAddr,
    token_id: string
}

interface ReceiveFtokenCallback {
    ftkn_instance: FtokenInstance,
}

interface Fractionalize {
    nft_info: UndrNftInfo,
    ftkn_init: FtokenInit,
}

export type FracHandleMsg = BatchReceiveNft | TransferNft | ReceiveFtokenCallback | Fractionalize;


export type FracHandleResponse = {};


/////////////////////////////////////////////////////////////////////////////////
// Query mesages
/////////////////////////////////////////////////////////////////////////////////


interface GetCount { }

export type FracQueryMsg = GetCount;


interface FracCountResponse {
    count: HumanAddr[],
}

//todo
export type FracQueryResponse = {};


/////////////////////////////////////////////////////////////////////////////////
// Private interfaces
/////////////////////////////////////////////////////////////////////////////////

interface FtokenInstance {
    ftkn_idx: u32,
    depositor: HumanAddr,
    ftoken_contr: ContractInfo,
    init_nft_info: UndrNftInfo,
    name: string,
    symbol: string,
    decimals: u8,
}

interface AucConf {
    bid_token: ContractInfo,
    auc_period: u64,
    resv_boundary: u32,
    min_bid_inc: u32,
    unlock_threshold: Uint128,
}

interface PropConf {
    min_stake: Uint128,
    vote_period: u64,
    vote_quorum: Uint128,
    veto_threshold: Uint128,
}

interface FtokenConf {
    min_ftkn_bond_prd: u64,
    priv_metadata_view_threshold: u32,
    auc_conf: AucConf,
    prop_conf: PropConf,
}

interface FtokenInit {
    name: string,
    symbol: string,
    supply: Uint128,
    decimals: u8,
    contract_label: string,
    init_resv_price: Uint128,
    ftkn_conf: FtokenConf,
}
