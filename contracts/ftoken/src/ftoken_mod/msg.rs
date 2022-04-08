use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    HumanAddr, Uint128, Env,
};

use fsnft_utils::{FtokenInstance, ContractInfo, UndrNftInfo, FtokenConf};
use secret_toolkit::{
    // serialization::{Json, Serde}, 
    utils::{HandleCallback, Query}, 
    snip721::{ViewerInfo, AccessLevel, Metadata, Expiration},
}; 
use crate::{
    contract::{RESPONSE_BLOCK_SIZE}, 
    msg::InitMsg,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitialBalance {
    pub address: HumanAddr,
    pub amount: Uint128,
}

/// Init Callback response to send upon instantiation of ftoken contract
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum InitRes {
    /// Callback to fractionalizer contract upon instantiation of ftoken contract
    ReceiveFtokenCallback {
        ftkn_instance: FtokenInstance,
    },
    /// set viewing key sent to nft contract
    SetViewingKey {
        /// desired viewing key
        key: String,
        /// optional message length padding
        padding: Option<String>,
    },
}

impl HandleCallback for InitRes {
    const BLOCK_SIZE: usize = RESPONSE_BLOCK_SIZE;
}

/// Implements register_receieve of ftoken contract on fractionalizer
impl InitRes {
    pub fn register_receive(msg: InitMsg, env: Env) -> Self {
        InitRes::ReceiveFtokenCallback {
            ftkn_instance: FtokenInstance {
                ftkn_idx: msg.init_info.ftkn_idx,
                depositor: msg.init_info.depositor,
                ftoken_contr: ContractInfo { 
                    code_hash: env.contract_code_hash, 
                    address: env.contract.address,
                },
                init_nft_info: UndrNftInfo {
                    token_id: msg.init_info.nft_info.token_id,
                    nft_contr: ContractInfo {
                        code_hash: msg.init_info.nft_info.nft_contr.code_hash,
                        address: msg.init_info.nft_info.nft_contr.address,
                    },
                },
                name: msg.name,
                symbol: msg.symbol,
                decimals: msg.decimals,
            }
        }
    }
}



/// List of messages that is allowed to be sent to underlying NFT
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AllowedNftMsg {
    SetMetadata {
        public_metadata: Option<Metadata>,
        private_metadata: Option<Metadata>,
    },
    Reveal { },
    MakeOwnershipPrivate { },
    SetGlobalApproval {
        view_owner: Option<AccessLevel>,
        view_private_metadata: Option<AccessLevel>,
        expires: Option<Expiration>,
    },
    SetWhitelistedApproval {
        address: HumanAddr,
        view_owner: Option<AccessLevel>,
        view_private_metadata: Option<AccessLevel>,
        expires: Option<Expiration>,
    },
}

/// List of messages that is allowed to be sent to underlying NFT
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum S721HandleMsg {
    /// set the public and/or private metadata.  This can be called by either the token owner or
    /// a valid minter if they have been given this power by the appropriate config values
    SetMetadata {
        /// id of the token whose metadata should be updated
        token_id: String,
        /// the optional new public metadata
        public_metadata: Option<Metadata>,
        /// the optional new private metadata
        private_metadata: Option<Metadata>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// Reveal the private metadata of a sealed token and mark the token as having been unwrapped
    Reveal {
        /// id of the token to unwrap
        token_id: String,
        /// optional message length padding
        padding: Option<String>,
    },
    /// if a contract was instantiated to make ownership public by default, this will allow
    /// an address to make the ownership of their tokens private.  The address can still use
    /// SetGlobalApproval to make ownership public either inventory-wide or for a specific token
    MakeOwnershipPrivate {
        /// optional message length padding
        padding: Option<String>,
    },
    /// add/remove approval(s) that whitelist everyone (makes public)
    SetGlobalApproval {
        /// optional token id to apply approval/revocation to
        token_id: Option<String>,
        /// optional permission level for viewing the owner
        view_owner: Option<AccessLevel>,
        /// optional permission level for viewing private metadata
        view_private_metadata: Option<AccessLevel>,
        /// optional expiration
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },
    /// add/remove approval(s) for a specific address on the token(s) you own.  Any permissions
    /// that are omitted will keep the current permission setting for that whitelist address
    SetWhitelistedApproval {
        /// address being granted/revoked permission
        address: HumanAddr,
        /// optional token id to apply approval/revocation to
        token_id: Option<String>,
        /// optional permission level for viewing the owner
        view_owner: Option<AccessLevel>,
        /// optional permission level for viewing private metadata
        view_private_metadata: Option<AccessLevel>,
        /// optional permission level for transferring
        transfer: Option<AccessLevel>,
        /// optional expiration
        expires: Option<Expiration>,
        /// optional message length padding
        padding: Option<String>,
    },
}

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

/// Proposal
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Proposal {
    MsgToNft {
        msg: AllowedNftMsg,
    },
    ChangeConfig {
        config: FtokenConf,
    },
}
