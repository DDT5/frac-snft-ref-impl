use std::any::type_name;

use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use cosmwasm_std::{HumanAddr, Uint128, Storage, ReadonlyStorage, StdResult, StdError, testing::mock_env, Env};

use secret_toolkit::{
    serialization::{Json, Serde}, 
    utils::Query, 
    snip721::ViewerInfo, 
};

/////////////////////////////////////////////////////////////////////////////////
// Structs for msgs between fractionalizer and ftoken contracts
/////////////////////////////////////////////////////////////////////////////////

/// Part of information sent from fractionalizer contract to ftoken contract on instantiation tx
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct FtokenContrInit {
    /// index of ftoken contract. Starts from 0 
    pub idx: u32,
    /// depositor of NFT into fractionalizer
    pub depositor: HumanAddr,
    /// contract hash of fractionalizer
    pub fract_hash: String,
    /// underlying NFT info
    pub nft_info: UndrNftInfo,
}

/// ftoken contract information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInfo {
    /// ftoken contract index from the fractionalizer contract's perspective 
    pub idx: u32,
    /// address which deposited the nft
    pub depositor: HumanAddr,
    /// code hash and address of ftoken contract
    pub ftoken_contr: ContractInfo,
    /// information on the underlying nft
    pub nft_info: UndrNftInfo,
    /// name of ftoken
    pub name: String,
    /// symbol of ftoken
    pub symbol: String,
    /// decimal of ftoken
    pub decimals: u8,
}

/// underlying NFT information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct UndrNftInfo {
    /// token id of underlying nft
    pub token_id: String,
    /// contract code hash and address of contract of underlying nft 
    pub nft_contr: ContractInfo,
}

/// Part of initialization message sent by USERS to fractionalizer 
/// initial configuration of fractionalized tokens
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenConf {
    /// name of the ftoken
    pub name: String,
    /// symbol of the ftoken
    pub symbol: String,
    /// supply in the lowest denomination
    pub supply: Uint128,
    /// determines the lowest denomination
    pub decimals: u8,
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address in HumanAddr
    pub address: HumanAddr,
}

/////////////////////////////////////////////////////////////////////////////////
// ftoken additions
/////////////////////////////////////////////////////////////////////////////////

/// Query messages to be sent to SNIP721 contract 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum S721QueryMsg {
    /// from snip721 contract
    /// display the owner of the specified token if authorized to view it.  If the requester
    /// is also the token's owner, the response will also include a list of any addresses
    /// that can transfer this token.  The transfer approval list is for CW721 compliance,
    /// but the NftDossier query will be more complete by showing viewing approvals as well
    OwnerOf {
        token_id: String,
        /// optional address and key requesting to view the token owner
        viewer: Option<ViewerInfo>,
        /// optionally include expired Approvals in the response list.  If ommitted or
        /// false, expired Approvals will be filtered out of the response
        include_expired: Option<bool>,
    },
}

impl Query for S721QueryMsg {
    const BLOCK_SIZE: usize = 256;
}


/////////////////////////////////////////////////////////////////////////////////
// json save, load and may_load, and remove
/////////////////////////////////////////////////////////////////////////////////

/// Returns StdResult<()> resulting from saving an item to storage using Json (de)serialization
/// because bincode2 annoyingly uses a float op when deserializing an enum
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn json_save<T: Serialize, S: Storage>(
    storage: &mut S,
    key: &[u8],
    value: &T,
) -> StdResult<()> {
    storage.set(key, &Json::serialize(value)?);
    Ok(())
}

/// Returns StdResult<T> from retrieving the item with the specified key using Json
/// (de)serialization because bincode2 annoyingly uses a float op when deserializing an enum.  
/// Returns a StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn json_load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Json::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

/// Returns StdResult<Option<T>> from retrieving the item with the specified key using Json
/// (de)serialization because bincode2 annoyingly uses a float op when deserializing an enum.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn json_may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Json::deserialize(&value).map(Some),
        None => Ok(None),
    }
}

/// Removes an item from storage. Named `json_remove` for consistency. Irrelevant
/// whether it uses json or bincode2 to de/serialize 
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn json_remove<S: Storage>(storage: &mut S, key: &[u8]) {
    storage.remove(key);
}


/////////////////////////////////////////////////////////////////////////////////
// multi unit test helpers
/////////////////////////////////////////////////////////////////////////////////

/// Serialized then deserializes a struct into / from json. Simulates a cosmos message being 
/// sent between contracts. Used for unit tests 
pub fn json_ser_deser<T: Serialize, U: DeserializeOwned>(value: &T) -> StdResult<U> {
    let ser = Json::serialize(value)?;
    let deser: U = Json::deserialize(&ser)?;
    Ok(deser)
}

pub fn more_mock_env(
    sender: HumanAddr,
    contract_addr: Option<HumanAddr>,
    contract_code_hash: Option<String>,
) -> Env {
    let mut env = mock_env(sender, &[]);
    if let Some(i) = contract_addr { env.contract.address = i }
    if let Some(i) = contract_code_hash { env.contract_code_hash = i }
    env
}
