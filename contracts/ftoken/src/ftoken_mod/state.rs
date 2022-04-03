use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Uint128, 
};
use cosmwasm_storage::{
    // PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton,
};
use crate::{
    viewing_key::ViewingKey
};
use fsnft_utils::{FtokenInfo, BidsInfo, FtokenConf};

pub const FTOKEN_CONTR_FTKN: &[u8] = b"ftkncontr_ftkn";
pub const FTKN_CONFIG: &[u8] = b"ftknconfig";
pub const FTKN_STAKE: &[u8] = b"ftknstake";
pub const ALLOWED_TOKENS: &[u8] = b"allowedtokens";
pub const BIDS_STORE: &[u8] = b"bidstore";
pub const PREFIX_UNDR_NFT: &[u8] = b"undrlynft";
pub const NFT_VIEW_KEY: &[u8] = b"nftviewkey";
pub const CURRENT_BID_ID: &[u8] = b"currentbidid";
pub const WON_BID_ID: &[u8] = b"wonbidid";
pub const VOTES_BUCKET: &[u8] = b"votesbucket";
pub const VOTES_TOTAL: &[u8] = b"votetotal";




/////////////////////////////////////////////////////////////////////////////////
// Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Bid storage: stores bid information
pub fn bids_w<S: Storage>(storage: &mut S) -> Bucket<S, BidsInfo> {
    bucket(BIDS_STORE, storage)
}
pub fn bids_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, BidsInfo> {
    bucket_read(BIDS_STORE, storage)
}

/// staked ftokens
pub fn ftkn_stake_w<S: Storage>(storage: &mut S) -> Bucket<S, StakedTokens> {
    bucket(FTKN_STAKE, storage)
}
pub fn ftkn_stake_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, StakedTokens> {
    bucket_read(FTKN_STAKE, storage)
}

/// vote tally, which is the running cumulative tally
pub fn votes_total_w<S: Storage>(storage: &mut S) -> Bucket<S, TotalVotes> {
    bucket(VOTES_TOTAL, storage)
}
pub fn votes_total_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, TotalVotes> {
    bucket_read(VOTES_TOTAL, storage)
}

/////////////////////////////////////////////////////////////////////////////////
// Multi-level Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Multilevel bucket to store votes. Key intended to be [`bid_id`, HumanAddr]  
pub fn votes_w<S: Storage>(
    storage: &mut S,
    bid_id: u32
) -> Bucket<S, VoteRegister> {
    Bucket::multilevel(&[VOTES_BUCKET, &bid_id.to_le_bytes()], storage)
}
pub fn votes_r<S: Storage>(
    storage: &S,
    bid_id: u32
) -> ReadonlyBucket<S, VoteRegister> {
    ReadonlyBucket::multilevel(&[VOTES_BUCKET, &bid_id.to_le_bytes()], storage)
}


/////////////////////////////////////////////////////////////////////////////////
// Singletons
/////////////////////////////////////////////////////////////////////////////////


/// FtokenContr storage: stores information on this ftokens contract
pub fn ftoken_info_w<S: Storage>(storage: &mut S) -> Singleton<S, FtokenInfo> {
    singleton(storage, FTOKEN_CONTR_FTKN)
}
pub fn ftoken_info_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, FtokenInfo> {
    singleton_read( storage, FTOKEN_CONTR_FTKN)
}

/// config specifically for ftoken functionality
pub fn ftkn_config_w<S: Storage>(storage: &mut S) -> Singleton<S, FtokenConf> {
    singleton(storage, FTKN_CONFIG)
}
pub fn ftkn_config_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, FtokenConf> {
    singleton_read( storage, FTKN_CONFIG)
}

/// index the next bid to be received 
pub fn bid_id_w<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage, CURRENT_BID_ID)
}
pub fn bid_id_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read( storage, CURRENT_BID_ID)
}

/// index the bid that won 
pub fn won_bid_id_w<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage, WON_BID_ID)
}
pub fn won_bid_id_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read( storage, WON_BID_ID)
}

/// stores viewing key to query nft contract
pub fn nft_vk_w<S: Storage>(storage: &mut S) -> Singleton<S, ViewingKey> {
    singleton(storage, NFT_VIEW_KEY)
}

pub fn nft_vk_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ViewingKey> {
    singleton_read(storage, NFT_VIEW_KEY)
}

/////////////////////////////////////////////////////////////////////////////////
// Structs
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakedTokens {
    pub amount: Uint128,
    pub unlock_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct VoteRegister {
    pub yes: Uint128,
    pub no: Uint128,
}

// impl Default for VoteRegister {
//     fn default() -> Self {
//         Self {
//             yes: Uint128(0),
//             no: Uint128(0),
//         }
//     }
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    Yes,
    No,
}

/// vote count, weighted by staked ftokens
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct TotalVotes {
    pub(crate) yes: Uint128,
    pub(crate) no: Uint128,
}

/////////////////////////////////////////////////////////////////////////////////
// to be removed eventually..
/////////////////////////////////////////////////////////////////////////////////

// /// stores ftoken information
// pub fn write_ftkn_info<S: Storage>(store: &mut S, val: &FtokenInfo) -> StdResult<()> {
//     json_save(store, FTOKEN_INFO, val)
// }

// pub fn read_ftkn_info<S: Storage>(store: &S) -> StdResult<FtokenInfo> {
//     json_load(store, FTOKEN_INFO)
// }

// /// stores viewing key to query nft contract
// pub fn write_nft_vk<S: Storage>(store: &mut S, val: &ViewingKey) -> StdResult<()> {
//     json_save(store, NFT_VIEW_KEY, val)
// }

// pub fn read_nft_vk<S: Storage>(store: &S) -> StdResult<ViewingKey> {
//     json_load(store, NFT_VIEW_KEY)
// }

// /// logs whether underlying nft is in the vault
// pub fn write_nft_in_vault<S: Storage>(store: &mut S, key: &UndrNftInfo, val: &bool) {
//     let mut store = PrefixedStorage::new(PREFIX_UNDR_NFT, store);
//     let ser_key = &ser_bin_data(key).unwrap();
//     let ser_val = &ser_bin_data(val).unwrap();
//     store.set(ser_key, ser_val);
// }

// pub fn read_nft_in_vault<S: Storage>(store: &S, key: &UndrNftInfo) -> bool {
//     let store = ReadonlyPrefixedStorage::new(PREFIX_UNDR_NFT, store);
//     let ser_key = &ser_bin_data(key).unwrap();
//     let ser_val = store.get(ser_key).unwrap();
//     deser_bin_data(&ser_val).unwrap()
// }