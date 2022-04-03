use std::any::{type_name};

use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use cosmwasm_std::{
    HumanAddr, Uint128, Storage, ReadonlyStorage, StdResult, StdError, CosmosMsg, WasmMsg, from_binary,
    Binary, Api, Querier, Extern, Env,
};
// use cosmwasm_std::testing::{mock_env};  // mock_dependencies, MockStorage, MockApi, MockQuerier,

use secret_toolkit::{
    serialization::{Json, Serde}, 
    utils::{Query, HandleCallback}, 
    snip721::ViewerInfo, 
};

pub const RESPONSE_BLOCK_SIZE: usize = 256;

/////////////////////////////////////////////////////////////////////////////////
// Structs for msgs between fractionalizer and ftoken contracts
/////////////////////////////////////////////////////////////////////////////////


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InterContrMsg{
    /// Receiver interface function for SNIP721 contract. Msg to be sent to SNIP721 contract
    /// register that the message sending contract implements ReceiveNft and possibly
    /// BatchReceiveNft.  If a contract implements BatchReceiveNft, SendNft will always
    /// call BatchReceiveNft even if there is only one token transferred (the token_ids
    /// Vec will only contain one ID)
    RegisterReceiveNft {
        /// receving contract's code hash
        code_hash: String,
        /// optionally true if the contract also implements BatchReceiveNft.  Defaults
        /// to false if not specified
        also_implements_batch_receive_nft: Option<bool>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// Message to send to SNIP721 contract
    TransferNft {
        recipient: HumanAddr, 
        token_id: String,
    },
    /// Message to send to SNIP721 contract
    SendNft {
        /// address to send the token to
        contract: HumanAddr,
        token_id: String,
        /// optional message to send with the (Batch)RecieveNft callback
        msg: Option<Binary>,
    },
    /// `Send` message to send to SNIP20 token address
    Send {
        recipient: HumanAddr,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
    /// `SendFrom` message to send to SNIP20 token address
    SendFrom {
        /// the address to send from
        owner: HumanAddr,
        recipient: HumanAddr,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
}

impl InterContrMsg {
    pub fn register_receive(code_hash: &String) -> Self {
        InterContrMsg::RegisterReceiveNft {
            code_hash: code_hash.to_string(),
            also_implements_batch_receive_nft: Some(true), 
            padding: None, // TODO add padding calculation
        }
    }
}

impl HandleCallback for InterContrMsg {
    const BLOCK_SIZE: usize = RESPONSE_BLOCK_SIZE;
}

/// ftoken contract information, stored in ftoken contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInfo {
    /// ftoken contract instance information, created at initialization
    pub instance: FtokenInstance,
    /// is underlying nft still in the vault (ie: fractionalized)
    pub in_vault: bool,
}

/// ftoken contract information created at initialization, stored directly in fractionalizer contract, also within
/// the FtokenInfo struct stored in ftoken contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInstance {
    /// ftoken contract index from the fractionalizer contract's perspective 
    pub ftkn_idx: u32,
    /// address which deposited the nft
    pub depositor: HumanAddr,
    /// code hash and address of ftoken contract
    pub ftoken_contr: ContractInfo,
    /// information on the underlying nft that was initially deposited
    pub init_nft_info: UndrNftInfo,
    /// name of ftoken
    pub name: String,
    /// symbol of ftoken
    pub symbol: String,
    /// decimal of ftoken
    pub decimals: u8,
}

/// Part of initialization message sent by USERS to fractionalizer 
/// initial configuration of fractionalized tokens
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInit {  //FtokenConf
    /// name of the ftoken
    pub name: String,
    /// symbol of the ftoken
    pub symbol: String,
    /// supply in the lowest denomination
    pub supply: Uint128,
    /// determines the lowest denomination
    pub decimals: u8,
    /// ftoken config which is stored in the ftoken contract
    pub ftkn_conf: FtokenConf,
}

/// Part of information sent from fractionalizer contract to ftoken contract on instantiation tx
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct FtokenContrInit {
    /// index of ftoken contract. Starts from 0 
    pub ftkn_idx: u32,
    /// depositor of NFT into fractionalizer
    pub depositor: HumanAddr,
    /// contract hash of fractionalizer
    pub fract_hash: String,
    /// underlying NFT info
    pub nft_info: UndrNftInfo,
    /// ftoken config which is stored in the ftoken contract
    pub ftkn_conf: FtokenConf,
}

/// ftoken config which is stored in the ftoken contract. 
/// Sent as init in fractionalize tx, and stored in ftoken contract 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct FtokenConf {
    pub bid_conf: BidConf,
}

/// ftoken config for bidding. Nested in a larger struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BidConf {
    /// determines the allowed bid token (eg: sSCRT)
    pub bid_token: ContractInfo,
    /// number of blocks that ftokens will be bonded after a vote. Important to prevent vote spamming and manipulation 
    pub min_ftkn_bond_prd: u64,
    /// number of blocks that a bid remains live before a finalize_vote_count tx can be called
    pub bid_period: u64,
    /// proportion of ftoken-weighted votes before quorum is reached. Unit in basis points (1/1000)
    pub bid_vote_quorum: Uint128,
}


/// underlying NFT information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct UndrNftInfo {
    /// token id of underlying nft
    pub token_id: String,
    /// contract code hash and address of contract of underlying nft 
    pub nft_contr: ContractInfo,
}

/// bids information as stored by ftoken contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BidsInfo {
    /// bid identifier
    pub bid_id: u32,
    pub bidder: HumanAddr,
    /// amount denominated in the approved bid token
    pub amount: Uint128,
    pub status: BidStatus,
    /// block height where bid has voting period ends. Final count tx can be called at this point forward
    pub end_height: u64,
}

/// Query messages to be sent to SNIP721 contract 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BidStatus {
    /// bid won, and nft has been retrieved
    WonRetrieved,
    /// bid won, but nft has not been retrieved
    WonInVault,
    /// bid still active
    Active,
    /// bid lost, bid amount still in treasury
    LostInTreasury,
    /// bid lost, bid amount has been retrieved back from treasury
    LostRetrieved,    
}

impl Default for BidStatus {
    fn default() -> Self {
        BidStatus::Active
    }
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
    const BLOCK_SIZE: usize = RESPONSE_BLOCK_SIZE;
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
// functions
/////////////////////////////////////////////////////////////////////////////////

/// Creates a `SendNft` cosmos msg to be sent to NFT contract 
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `contract` - HumanAddr (String) of receiver of nft, ie: ftoken contract address
pub fn send_nft_msg<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    nft_contr_addr: HumanAddr,
    nft_contr_hash: String,
    contract: HumanAddr,
    token_id: String,
    msg: Option<Binary>,
) -> StdResult<CosmosMsg> {

    let contract_msg = InterContrMsg::SendNft {
            // address of recipient of nft
            contract, 
            token_id,
            msg,
        };

    let cosmos_msg = contract_msg.to_cosmos_msg(
        nft_contr_hash, 
        HumanAddr(nft_contr_addr.to_string()), 
        None
    )?;

    Ok(cosmos_msg)

    // // create messages
    // let messages = vec![cosmos_msg];

    // Ok(HandleResponse {
    //     messages,
    //     log: vec![],
    //     data: None
    // })
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

/// extracts the msg from a CosmosMsg. uses DeserializedOwned, so you need to specify the type
/// when declaring the variable.
/// # Usage
/// let var: `type here` = extract_cosmos_msg(&message).unwrap();
pub fn extract_cosmos_msg<U: DeserializeOwned>(message: &CosmosMsg) -> StdResult<U> {
    // let mut decode: String; 
    let msg = match message {
        CosmosMsg::Wasm(i) => match i {
            WasmMsg::Execute{msg, ..} => msg,
            WasmMsg::Instantiate {msg, ..} => msg,
        },
        _ => return Err(StdError::generic_err("unable to extract msg from CosmosMsg"))
    };
    let decoded: U = from_binary(&msg).unwrap();
    Ok(decoded)
}


// pub fn more_mock_env(
//     sender: HumanAddr,
//     contract_addr: Option<HumanAddr>,
//     contract_code_hash: Option<String>,
// ) -> Env {
//     let mut env = mock_env(sender, &[]);
//     if let Some(i) = contract_addr { env.contract.address = i }
//     if let Some(i) = contract_code_hash { env.contract_code_hash = i }
//     env
// }
