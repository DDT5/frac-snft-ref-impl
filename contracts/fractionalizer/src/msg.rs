use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Binary, HumanAddr, Uint128};
use secret_toolkit::utils::{InitCallback, HandleCallback};  

use crate::{contract::{BLOCK_SIZE}, state::UploadedFtkn};

use fsnft_utils::{FtokenContrInit, FtokenInfo, FtokenConf, UndrNftInfo};

/////////////////////////////////////////////////////////////////////////////////
// Init message
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub uploaded_ftoken: UploadedFtkn,
}


/////////////////////////////////////////////////////////////////////////////////
// Handle messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Function for users to call. When called, fsnft contract will register with SNIP721 contract
    // RegisterNftContr {
    //     /// The SNIP721 contract code hash
    //     reg_hash: String,
    //     /// The SNIP721 contract address to register with
    //     reg_addr: HumanAddr,
    // },
    /// Receiver interface function for SNIP721 contract. Msg to be received from SNIP721 contract
    /// BatchReceiveNft may be a HandleMsg variant of any contract that wants to implement a receiver
    /// interface.  BatchReceiveNft, which is more informative and more efficient, is preferred over
    /// ReceiveNft.
    BatchReceiveNft {
        /// address that sent the tokens.  There is no ReceiveNft field equivalent to this
        sender: HumanAddr,
        /// previous owner of sent tokens.  This is equivalent to the ReceiveNft `sender` field
        from: HumanAddr,
        /// tokens that were sent
        token_ids: Vec<String>,
        /// optional message to control receiving logic
        msg: Option<Binary>,
    },
    /// Transfers an NFT owned by this contract
    TransferNft {
        nft_contr_addr: HumanAddr,
        nft_contr_hash: String,
        recipient: HumanAddr,
        token_id: String
    },
    /// `send` an NFT that this contract has permission to transfer
    // SendNft {
    //     nft_contr_addr: HumanAddr,
    //     nft_contr_hash: String,
    //     /// address to send the token to
    //     contract: HumanAddr,
    //     token_id: String,
    //     msg: Option<Binary>,
    // },
    /// Msg sent by user to instantiate ftoken contract
    // InstantiateFtoken {
    //     name: String,
    //     symbol: String,
    //     decimals: u8,
    //     callback_code_hash: String,
    // },
    /// Receiver for InitResponse callback from ftoken contract  
    ReceiveFtokenCallback {
        ftoken_contr: FtokenInfo,
    },
    /// User calls this function to fractionalize an NFT
    /// User must first give permission to fractionalizer to transfer the NFT
    Fractionalize {
        /// Underlying NFT information
        /// token id and SNIP721 contract address and hash
        nft_info: UndrNftInfo,
        /// configuration of fractionalized token
        ftkn_conf: FtokenConf,
    },
}


// ------------------------------------------------------------------------------
// Enums and structs (init) for callback
// ------------------------------------------------------------------------------

/// From SNIP20 ref impl, added [derive(Debug)]
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitialBalance {
    pub address: HumanAddr,
    pub amount: Uint128,
}

/// From SNIP20 ref impl, added [derive(PartialEq)]
/// This type represents optional configuration values which can be overridden.
/// All values are optional and have defaults which are more private by default,
/// but can be overridden if necessary
#[derive(Serialize, Deserialize, JsonSchema, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InitConfig {
    /// Indicates whether the total supply is public or should be kept secret.
    /// default: False
    public_total_supply: Option<bool>,
    /// Indicates whether deposit functionality should be enabled
    /// default: False
    enable_deposit: Option<bool>,
    /// Indicates whether redeem functionality should be enabled
    /// default: False
    enable_redeem: Option<bool>,
    /// Indicates whether mint functionality should be enabled
    /// default: False
    enable_mint: Option<bool>,
    /// Indicates whether burn functionality should be enabled
    /// default: False
    enable_burn: Option<bool>,
}


/// Msg sent to ftoken contract to instantiate it
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitFtoken {
    /// fractionalizer contract hash and address
    pub init_info: FtokenContrInit,
    pub name: String,
    pub admin: Option<HumanAddr>,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Option<Vec<InitialBalance>>,
    pub prng_seed: Binary,
    pub config: Option<InitConfig>,
}

impl InitCallback for InitFtoken {
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

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
    const BLOCK_SIZE: usize = BLOCK_SIZE;
}

/////////////////////////////////////////////////////////////////////////////////
// Query messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetCount {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CountResponse {
    pub count: Vec<HumanAddr>,
}
