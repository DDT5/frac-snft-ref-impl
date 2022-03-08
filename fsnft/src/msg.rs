use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Binary, HumanAddr};
use secret_toolkit::utils::{HandleCallback};  

use crate::contract::{BLOCK_SIZE};


/////////////////////////////////////////////////////////////////////////////////
// Init message
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
}


/////////////////////////////////////////////////////////////////////////////////
// Handle messages
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Function for users to call. When called, fsnft contract will register with SNIP721 contract
    Register {
        /// The SNIP721 contract address to registered with
        reg_addr: HumanAddr,
        /// The SNIP721 contract code hash
        reg_hash: String,
    },
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
    // Transfers an NFT owned by this contract
    TransferNft {
        nft_contr_addr: HumanAddr,
        nft_contr_hash: String,
        recipient: HumanAddr,
        token_id: String
    },
    // `send` an NFT that this contract has permission to transfer
    SendNft {
        nft_contr_addr: HumanAddr,
        nft_contr_hash: String,
        /// address to send the token to
        contract: HumanAddr,
        token_id: String,
        msg: Option<Binary>,
    }
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub enum HandleAnswer {
//     Rn {
//         rn: [u8; 32],
//     },
// }

// impl HandleCallback for HandleAnswer {
//     const BLOCK_SIZE: usize = BLOCK_SIZE;
// }


// ------------------------------------------------------------------------------
// Enums for callback
// ------------------------------------------------------------------------------

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
    pub fn register_receive(code_hash: String) -> Self {
        InterContrMsg::RegisterReceiveNft {
            code_hash,
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
