use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage}; //CanonicalAddr
use cosmwasm_storage::{
    // PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton,
};

use fsnft_utils::{FtokenInstance};

pub const CONFIG_KEY: &[u8] = b"config";
pub const PENDING_REG: &[u8] = b"pendreg";
pub const FTKN_INDEX: &[u8] = b"ftknidx";
pub const UPLOADED_FTKN: &[u8] = b"uploadftkn";
pub const FTOKEN_CONTR_FRAC: &[u8] = b"ftkncontr_frac";



/////////////////////////////////////////////////////////////////////////////////
// Buckets
/////////////////////////////////////////////////////////////////////////////////

/// FtokenContr storage: stores information on the ftokens that fractionalizer contract
/// has created
pub fn ftoken_instance_w<S: Storage>(storage: &mut S) -> Bucket<S, FtokenInstance> {
    bucket(FTOKEN_CONTR_FRAC, storage)
}
//ftoken_contr_r
pub fn ftoken_instance_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, FtokenInstance> {
    bucket_read(FTOKEN_CONTR_FRAC, storage)
}


/////////////////////////////////////////////////////////////////////////////////
// Singletons
/////////////////////////////////////////////////////////////////////////////////

pub fn config_w<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_KEY)
}
pub fn config_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, Config> {
    singleton_read(storage, CONFIG_KEY)
}

/// index of next ftoken contract to be created
pub fn ftkn_idx_w<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage,FTKN_INDEX)
}
pub fn ftkn_idx_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read(storage, FTKN_INDEX)
}

/// pending info (HumanAddr of depositor) of ftoken contract to be registered, so fractionalizer 
/// can verify the callback from ftoken contract. Info should not last beyond a transaction 
pub fn pending_reg_w<S: Storage>(storage: &mut S) -> Singleton<S, HumanAddr> {
    singleton(storage,PENDING_REG)
}
pub fn pending_reg_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, HumanAddr> {
    singleton_read(storage, PENDING_REG)
}

/// stores the code_id and code hash of the ftoken contract code that has been uploaded 
pub fn ftkn_id_hash_w<S: Storage>(storage: &mut S) -> Singleton<S, UploadedFtkn> {
    singleton(storage,UPLOADED_FTKN)
}
pub fn ftkn_id_hash_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, UploadedFtkn> {
    singleton_read(storage, UPLOADED_FTKN)
}


/////////////////////////////////////////////////////////////////////////////////
// Structs
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub known_snip_721: Vec<HumanAddr>,
}

/// the code_id and code hash of the ftoken contract code that has been uploaded 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct UploadedFtkn {
    /// code_id of uploaded ftoken contract
    pub code_id: u64,
    /// code hash of uploaded ftoken contract
    pub code_hash: String,
}

