use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{
    HumanAddr, Uint128, Storage, StdResult, CosmosMsg,
    Binary, Api, Querier, Extern, Env,
};
// use cosmwasm_std::testing::{mock_env};  // mock_dependencies, MockStorage, MockApi, MockQuerier,

use secret_toolkit::{
    utils::{HandleCallback}, 
};

pub const RESPONSE_BLOCK_SIZE: usize = 256;

/////////////////////////////////////////////////////////////////////////////////
// Intercontract messages
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
    /// `Transfer` message to send to SNIP20 token address
    Transfer {
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    /// `TransferFrom` message to send to SNIP20 token address
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    }
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


/////////////////////////////////////////////////////////////////////////////////
// States
/////////////////////////////////////////////////////////////////////////////////

/// ftoken overall config which is stored in the ftoken contract. 
/// Sent as init in fractionalize tx, and stored in ftoken contract 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct FtokenConf {
    /// Number of blocks that ftokens will be bonded after a vote (on reservation
    /// price or on proposals). Important to prevent vote spamming and manipulation 
    pub min_ftkn_bond_prd: u64,
    /// Proportion of ftoken ownership required before private metadata of underlying
    /// NFT can be queried by ftoken owner. This needs to be done with authenticated
    /// query, either through viewing keys or viewing permit. Unit in basis points (ie:
    /// 1/10_000)
    pub priv_metadata_view_threshold: u32,
    /// Configurations for auctions
    pub auc_conf: AucConf,
    /// Configurations for proposals
    pub prop_conf: PropConf,
}

/// ftoken config for bidding. Nested in a larger struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct AucConf {
    /// Determines the token that bids are made in (eg: sSCRT)
    pub bid_token: ContractInfo,
    /// Number of blocks that a bid remains live before a finalize_vote_count tx can be called
    pub auc_period: u64,
    /// User needs to vote a reservation price within this boundary. Boundary is the percentage above and below
    /// current reservation price.
    /// Floor = `current reservation price` * 100 / `minmax_boundary`.
    /// Ceiling = `current reservation price` * `minmax_boundary` / 100.
    pub resv_boundary: u32,
    /// Min bid increment proportion in basis points ie: 1/10_000. So a setting of 10 means that if the current highest bid
    /// is 100_000 tokens, the next bid needs to be at least 1/1000 higher, or 100_100 tokens  
    pub min_bid_inc: u32,
    /// Proportion of ftoken OF TOTAL SUPPLY before NFT gets unlocked. Unit in basis points (1/1000)
    pub unlock_threshold: Uint128,
}

/// ftoken contract config for dao proposals. Nested in a larger struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct PropConf {
    /// Minimum ftoken stake to make a proposal
    pub min_stake: Uint128,
    /// Number of blocks that a proposal remains live before a finalization tx can be called
    pub vote_period: u64,
    /// Proportion of ftoken-weighted votes OF TOTAL SUPPLY before quorum is reached. Unit in basis points (1/1000)
    pub vote_quorum: Uint128,
    /// Proportion of ftoken-weighted votes OF TOTAL SUPPLY that needs to vote `veto` for a veto to apply. Unit in basis points (1/1000)
    pub veto_threshold: Uint128,
}

/// ftoken contract information, stored in ftoken contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInfo {
    /// ftoken contract instance information, created at initialization
    pub instance: FtokenInstance,
    /// Is underlying nft still in the vault (ie: fractionalized)
    pub vault_active: bool,
}

/// ftoken contract information created at initialization, stored directly in fractionalizer contract, also within
/// the FtokenInfo struct stored in ftoken contracts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInstance {
    /// ftoken contract index from the fractionalizer contract's perspective 
    pub ftkn_idx: u32,
    /// Address which deposited the nft
    pub depositor: HumanAddr,
    /// Code hash and address of ftoken contract
    pub ftoken_contr: ContractInfo,
    /// Information on the underlying nft that was initially deposited
    pub init_nft_info: UndrNftInfo,
    /// Name of ftoken
    pub name: String,
    /// Symbol of ftoken
    pub symbol: String,
    /// Decimal of ftoken
    pub decimals: u8,
}

/// Part of initialization message sent by USERS to fractionalizer 
/// initial configuration of fractionalized tokens
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenInit {  //FtokenConf
    /// Name of the ftoken
    pub name: String,
    /// Symbol of the ftoken
    pub symbol: String,
    /// Supply in the lowest denomination
    pub supply: Uint128,
    /// Determines the lowest denomination
    pub decimals: u8,
    /// Label String of the ftoken contract which will be instantiated. Instantiation of the new ftoken
    /// contract will fail if the label already exists on another contract on Secret Network 
    pub contract_label: String, 
    /// Initial reservation price which determines the initial min and max reservation price vote
    /// for the first user who votes on reservation price
    pub init_resv_price: Uint128,
    /// ftoken config which is stored in the ftoken contract
    pub ftkn_conf: FtokenConf,
}

/// Part of information sent from fractionalizer contract to ftoken contract on instantiation tx
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct FtokenContrInit {
    /// Index of ftoken contract. Starts from 0 
    pub ftkn_idx: u32,
    /// Depositor of NFT into fractionalizer
    pub depositor: HumanAddr,
    /// Contract hash of fractionalizer
    pub fract_hash: String,
    /// Underlying NFT info
    pub nft_info: UndrNftInfo,
    /// Initial reservation price which determines the initial min and max reservation price vote
    /// for the first user who votes on reservation price
    pub init_resv_price: Uint128,
    /// ftoken config which is stored in the ftoken contract
    pub ftkn_conf: FtokenConf,
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ContractInfo {
    /// Contract's code hash string
    pub code_hash: String,
    /// Contract's address in HumanAddr
    pub address: HumanAddr,
}

/// underlying NFT information
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct UndrNftInfo {
    /// Token id of underlying nft
    pub token_id: String,
    /// Contract code hash and address of contract of underlying nft 
    pub nft_contr: ContractInfo,
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
}

