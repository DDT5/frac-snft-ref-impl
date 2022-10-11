import { ContractInfo, HumanAddr, Uint128, Binary, u8, u32, u64 } from "./utils";

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

export default interface FtokenConf {
    min_ftkn_bond_prd: u64,
    priv_metadata_view_threshold: u32,
    auc_conf: AucConf,
    prop_conf: PropConf,
}