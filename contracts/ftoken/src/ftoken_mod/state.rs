use cosmwasm_std::{
    Storage, 
};
use cosmwasm_storage::{
    // PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton,
};
use crate::{
    viewing_key::ViewingKey
};
use fsnft_utils::{FtokenInfo, ContractInfo, BidsInfo};

pub const FTOKEN_CONTR_FTKN: &[u8] = b"ftkncontr_ftkn";
pub const ALLOWED_TOKENS: &[u8] = b"allowedtokens";
pub const BIDS_STORE: &[u8] = b"bidstore";
pub const PREFIX_UNDR_NFT: &[u8] = b"undrlynft";
pub const NFT_VIEW_KEY: &[u8] = b"nftviewkey";
pub const CURRENT_BID_ID: &[u8] = b"currentbidid";
pub const WON_BID_ID: &[u8] = b"wonbidid";



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

// pub fn multilevel_bucket<S: Storage>(
//     storage: &mut S,
//     id: u8
// ) -> Bucket<S, String> {
//     let bucket: Bucket<_,String> = Bucket::multilevel(&[FTOKEN_CONTR, &id.to_le_bytes()], storage);
//     bucket
// }


/////////////////////////////////////////////////////////////////////////////////
// Singletons
/////////////////////////////////////////////////////////////////////////////////


/// FtokenContr storage: stores information on this ftokens contract, which would 
/// have been created by the fractionalizer contract
/// _s_ stands for singleton, to differentiate vs. the function in the fractionalizer contract
pub fn ftoken_contr_s_w<S: Storage>(storage: &mut S) -> Singleton<S, FtokenInfo> {
    singleton(storage, FTOKEN_CONTR_FTKN)
}
pub fn ftoken_contr_s_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, FtokenInfo> {
    singleton_read( storage, FTOKEN_CONTR_FTKN)
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


/// stores ContractInfo of allowed bid tokens
pub fn allowed_bid_tokens_w<S: Storage>(storage: &mut S) -> Singleton<S, ContractInfo> {
    singleton(storage,ALLOWED_TOKENS)
}
pub fn allowed_bid_tokens_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ContractInfo> {
    singleton_read(storage, ALLOWED_TOKENS)
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