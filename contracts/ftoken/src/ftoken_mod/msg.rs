use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    HumanAddr, Env, Uint128,
};

use fsnft_utils::{
    FtokenInstance, ContractInfo, UndrNftInfo, FtokenInfo, FtokenConf,
    AucConf, PropConf,
};
use secret_toolkit::{
    // serialization::{Json, Serde}, 
    utils::{HandleCallback, Query}, 
    snip721::{ViewerInfo, AccessLevel, Metadata, Expiration, NftDossier,},
    permit::Permit,
}; 
use crate::{
    contract::{RESPONSE_BLOCK_SIZE}, 
    msg::InitMsg,
};

use super::{
    state::{StakedTokens, ResvVote, PropInfoTally, VoteRegister, BidInfo},
};

/////////////////////////////////////////////////////////////////////////////////
// Instantiation
/////////////////////////////////////////////////////////////////////////////////

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


/////////////////////////////////////////////////////////////////////////////////
// ftoken query messages
/////////////////////////////////////////////////////////////////////////////////

/// Public (ie: non authenticated) query messages that are specific to ftoken 
/// functionality (as opposed to standard SNIP20 queries)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FtokenQuery {
    /// Information on the ftoken determined at the point of fractionalization
    FtokenInfo { },
    /// ftoken configuration, including auction and DAO parameters. These configurations
    /// can be changed through a DAO
    FtokenConfig { },
    AuctionConfig { },
    ProposalConfig { },
    /// The minimum amount that a bidder needs to bid (to buy out the underlying NFT) in 
    /// order for the bid to be valid.
    ReservationPrice { },
    /// List of DAO proposals 
    ProposalList { },
    // Enabling this reduces the privacy of bidders. Blockchain analysis or side chain attacks
    // can easily reveal address of bidders
    BidList { 
        page: u32, 
        page_size: u32 
    },
}

/// Authenticated queries (ie: required viewing key or query permit) that are specific
/// to ftoken functionality (as opposed to standard SNIP20 queries)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FtokenAuthQuery {
    NftPrivateMetadata { },
    NftDossier { },
    StakedTokens { },
    ReservationPriceVote { },
    ProposalVotes { prop_id: u32 },
    Bid { },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FtokenQueryAnswer {
    FtokenInfo {
        ftkn_info: FtokenInfo,
    },
    FtokenConfig { 
        ftkn_conf: FtokenConf,
    },
    AuctionConfig { 
        auc_conf: AucConf,
    },
    ProposalConfig { 
        prop_conf: PropConf,
    },
    ReservationPrice { 
        ftokens_voted: Uint128,
        reservation_price: Uint128,
    },
    ProposalList(Vec<PropInfoTally>),
    BidList { 
        bid_amounts: Vec<Uint128>,
        total_bids: u64,
    },
    NftPrivateMetadata(PrivateMetadataResponse),
    NftDossier(NftDossierResponse),
    StakedTokens(StakedTokens),
    ReservationPriceVote(ResvVote),
    ProposalVotes(VoteRegister),
    Bid(BidInfo),
}


/////////////////////////////////////////////////////////////////////////////////
// SNIP1155 permit
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Snip1155Permissions {
    /// Allowance for SNIP-20 - Permission to query allowance of the owner & spender
    Allowance,
    /// Balance for SNIP-20 - Permission to query balance
    Balance,
    /// History for SNIP-20 - Permission to query transfer_history & transaction_hisotry
    History,
    /// Owner permission indicates that the bearer of this permit should be granted all
    /// the access of the creator/signer of the permit.  SNIP-721 uses this to grant
    /// viewing access to all data that the permit creator owns and is whitelisted for.
    /// For SNIP-721 use, a permit with Owner permission should NEVER be given to
    /// anyone else.  If someone wants to share private data, they should whitelist
    /// the address they want to share with via a SetWhitelistedApproval tx, and that
    /// address will view the data by creating their own permit with Owner permission
    Owner,
    /// For ftokens: PrivateMetadata of underlying NFT
    NftPrivateMetadata,
    /// For ftokens: NftDossier of underlying NFT
    NftDossier,
    /// For ftokens: Staked ftokens associated with the address
    StakedTokens,
    /// For ftokens: Reservation price vote by the address
    ReservationPriceVote,
    /// For ftokens: Votes on proposals by the address
    ProposalVotes,
    /// For ftokens: Bids made by the address
    Bid,
}

pub type Snip1155Permit = Permit<Snip1155Permissions>;

/////////////////////////////////////////////////////////////////////////////////
// Messages between ftoken contract and underlying NFT
/////////////////////////////////////////////////////////////////////////////////

/// List of messages that is allowed to be sent to underlying NFT. ftoken holders
/// can propose to send these messages to the underlying NFT, where other ftoken 
/// holders vote on whether to accept the proposal. Once a proposal passes, a  
/// transaction can be triggered to send the proposed message to the underlying NFT
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

/// Query messages to be sent to SNIP721 contract. Uses viewing key for cross
/// contract query, rather than permits
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
    /// displays the private metadata if permitted to view it
    PrivateMetadata {
        token_id: String,
        /// optional address and key requesting to view the private metadata
        viewer: Option<ViewerInfo>,
    },
    /// displays all the information about a token that the viewer has permission to
    /// see.  This may include the owner, the public metadata, the private metadata, royalty
    /// information, mint run information, whether the token is unwrapped, whether the token is
    /// transferable, and the token and inventory approvals
    NftDossier {
        token_id: String,
        /// optional address and key requesting to view the token information
        viewer: Option<ViewerInfo>,
        /// optionally include expired Approvals in the response list.  If ommitted or
        /// false, expired Approvals will be filtered out of the response
        include_expired: Option<bool>,
    },
}

impl Query for S721QueryMsg {
    const BLOCK_SIZE: usize = RESPONSE_BLOCK_SIZE;
}

/// wrapper to deserialize `PrivateMetadata` responses, with additional implementations
/// /// above the standard implementation in `secret_toolkit`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PrivateMetadataResponse {
    pub private_metadata: Metadata,
}

/// wrapper to deserialize `NftDossier` responses, with additional implementations
/// above the standard implementation in `secret_toolkit`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NftDossierResponse {
    pub nft_dossier: NftDossier,
}

/// DAO proposals that an ftoken holder can make. A minimum amount of tokens
/// need to be staked along with proposals 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Proposal {
    /// Proposal to send a message to the underlying NFT
    MsgToNft {
        msg: AllowedNftMsg,
    },
    /// Proposals to change the ftoken configuration, which includes auction
    /// configurations and DAO configurations
    ChangeConfig {
        config: FtokenConf,
    },
}
