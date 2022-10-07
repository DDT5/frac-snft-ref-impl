#![allow(clippy::field_reassign_with_default)] // This is triggered in `#[derive(JsonSchema)]`

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::batch;
use crate::transaction_history::{RichTx, Tx};
use crate::viewing_key::ViewingKey;
use cosmwasm_std::{
    Binary, HumanAddr, StdError, StdResult, Uint128,
};
// ftoken removals:
// use secret_toolkit::permit::Permit;

// ftoken additions:
use fsnft_utils::{FtokenContrInit, FtokenInfo, FtokenConf};
// use crate::{
//     receiver::Snip20ReceiveMsg,
// };
use crate::ftoken_mod::{
    msg::{Proposal, FtokenQuery, FtokenAuthQuery, FtokenQueryAnswer},
    state::{Vote},
};

// use crate::ftoken_mod::ft_permit::Permit;
use crate::ftoken_mod::msg::Snip1155Permit;

// ///////////////////////////


#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitialBalance {
    pub address: HumanAddr,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)] // ftoken addition: Clone
pub struct InitMsg {
    /// ftoken addition: fractionalizer contract hash and idx
    pub init_info: FtokenContrInit,
    pub name: String,
    pub admin: Option<HumanAddr>,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Option<Vec<InitialBalance>>,
    pub prng_seed: Binary,
    pub config: Option<InitConfig>,
}

impl InitMsg {
    pub fn config(&self) -> InitConfig {
        self.config.clone().unwrap_or_default()
    }
}

/// This type represents optional configuration values which can be overridden.
/// All values are optional and have defaults which are more private by default,
/// but can be overridden if necessary
#[derive(Serialize, Deserialize, JsonSchema, Clone, Default, Debug)]
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

impl InitConfig {
    pub fn public_total_supply(&self) -> bool {
        self.public_total_supply.unwrap_or(false)
    }

    pub fn deposit_enabled(&self) -> bool {
        self.enable_deposit.unwrap_or(false)
    }

    pub fn redeem_enabled(&self) -> bool {
        self.enable_redeem.unwrap_or(false)
    }

    pub fn mint_enabled(&self) -> bool {
        self.enable_mint.unwrap_or(false)
    }

    pub fn burn_enabled(&self) -> bool {
        self.enable_burn.unwrap_or(false)
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // Native coin interactions
    Redeem {
        amount: Uint128,
        denom: Option<String>,
        padding: Option<String>,
    },
    Deposit {
        padding: Option<String>,
    },

    // Base ERC-20 stuff
    Transfer {
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    Send {
        recipient: HumanAddr,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchTransfer {
        actions: Vec<batch::TransferAction>,
        padding: Option<String>,
    },
    BatchSend {
        actions: Vec<batch::SendAction>,
        padding: Option<String>,
    },
    Burn {
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    RegisterReceive {
        code_hash: String,
        padding: Option<String>,
    },
    CreateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
    SetViewingKey {
        key: String,
        padding: Option<String>,
    },

    // Allowance
    IncreaseAllowance {
        spender: HumanAddr,
        amount: Uint128,
        expiration: Option<u64>,
        padding: Option<String>,
    },
    DecreaseAllowance {
        spender: HumanAddr,
        amount: Uint128,
        expiration: Option<u64>,
        padding: Option<String>,
    },
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    SendFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        recipient_code_hash: Option<String>,
        amount: Uint128,
        msg: Option<Binary>,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchTransferFrom {
        actions: Vec<batch::TransferFromAction>,
        padding: Option<String>,
    },
    BatchSendFrom {
        actions: Vec<batch::SendFromAction>,
        padding: Option<String>,
    },
    BurnFrom {
        owner: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchBurnFrom {
        actions: Vec<batch::BurnFromAction>,
        padding: Option<String>,
    },

    // Mint
    Mint {
        recipient: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        padding: Option<String>,
    },
    BatchMint {
        actions: Vec<batch::MintAction>,
        padding: Option<String>,
    },
    AddMinters {
        minters: Vec<HumanAddr>,
        padding: Option<String>,
    },
    RemoveMinters {
        minters: Vec<HumanAddr>,
        padding: Option<String>,
    },
    SetMinters {
        minters: Vec<HumanAddr>,
        padding: Option<String>,
    },

    // Admin
    ChangeAdmin {
        address: HumanAddr,
        padding: Option<String>,
    },
    SetContractStatus {
        level: ContractStatusLevel,
        padding: Option<String>,
    },

    // Permit
    RevokePermit {
        permit_name: String,
        padding: Option<String>,
    },

    /// ftoken additions: 
    /// SNIP721 receiver
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
    /// Bidder calls this function to place a bid for underlying NFT. Bid needs to be at least as
    /// large as the reservation price, otherwise the tx is invalid
    Bid {
        /// bid amount denominated in the smallest denomination of the token
        amount: Uint128
    },

    // /// Receiver interface for sSCRT contract's `SendFrom` callback
    // Receive(Snip20ReceiveMsg),
    /// Stake ftokens. Ftoken holders need to stake ftokens before voting on either
    /// proposals or the reservation price. Staked ftokens will be bonded for a period
    /// specified by the configuration 
    Stake {
        /// The amount that the ftoken holder wishes to stake
        amount: Uint128,
    },
    /// Unstake ftokens. Ftoken holders can unstake ftokens after the bonding period
    Unstake {
        /// The amount that the ftoken holder wishes to unstake
        amount: Uint128,
    },
    /// Votes for DAO proposals to change configuration of the ftokens or send messages
    /// to the underlying NFT
    VoteProposal {
        /// The proposal ID
        prop_id: u32,
        /// The vote that the user wishes to cast on the proposal
        vote: Vote,
    },
    FinalizeAuction { },
    /// Message bidder calls to retrieve underlying NFT after winning a bid
    // RetrieveNft {
    //     /// the underlying nft token idx, which can be obtained via query (todo)
    //     bid_id: u32,
    // },
    /// If a bidder fails to win an auction, the bidder can retrieve its bid after the 
    /// auction period if over
    RetrieveBid { },
    /// Once an underlying NFT is bought out, ftoken holders can claim their pro-rata
    /// share of sales proceeds
    ClaimProceeds { },
    /// Make a DAO proposal
    Propose {
        proposal: Proposal,
        /// Users need to stake a certain number of ftokens when making proposals, which 
        /// is set initially and can be later configured through the DAO. This stake can
        /// be retrieve once the proposal's voting period is over and the outcome finalized.
        /// Proposers lose their stake if their proposal outcome is `LostWithVeto`
        stake: Uint128,
    },
    /// Once a proposal reaches the end of its voting period, anyone may call this 
    /// transaction to simultaneously finalize the vote count and execute the proposal
    FinalizeExecuteProp {
        prop_id: u32,
    },
    /// Proposers may retrieve their staked ftokens after proposals are finalized and
    /// executed. This stake is lost if the vote outcome is `LostWithVeto`
    RetrievePropStake {
        prop_id: u32,
    },
    /// ftoken holders whic have staked ftokens may vote a reservation price for 
    /// the underlying NFT. The weighted average votes on reservation price will be 
    /// the minimum bid amount that bidders need to make to buy out the underlying NFT
    VoteReservationPrice {
        resv_price: Uint128,
    }

}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    // Native
    Deposit {
        status: ResponseStatus,
    },
    Redeem {
        status: ResponseStatus,
    },

    // Base
    Transfer {
        status: ResponseStatus,
    },
    Send {
        status: ResponseStatus,
    },
    BatchTransfer {
        status: ResponseStatus,
    },
    BatchSend {
        status: ResponseStatus,
    },
    Burn {
        status: ResponseStatus,
    },
    RegisterReceive {
        status: ResponseStatus,
    },
    CreateViewingKey {
        key: ViewingKey,
    },
    SetViewingKey {
        status: ResponseStatus,
    },

    // Allowance
    IncreaseAllowance {
        spender: HumanAddr,
        owner: HumanAddr,
        allowance: Uint128,
    },
    DecreaseAllowance {
        spender: HumanAddr,
        owner: HumanAddr,
        allowance: Uint128,
    },
    TransferFrom {
        status: ResponseStatus,
    },
    SendFrom {
        status: ResponseStatus,
    },
    BatchTransferFrom {
        status: ResponseStatus,
    },
    BatchSendFrom {
        status: ResponseStatus,
    },
    BurnFrom {
        status: ResponseStatus,
    },
    BatchBurnFrom {
        status: ResponseStatus,
    },

    // Mint
    Mint {
        status: ResponseStatus,
    },
    BatchMint {
        status: ResponseStatus,
    },
    AddMinters {
        status: ResponseStatus,
    },
    RemoveMinters {
        status: ResponseStatus,
    },
    SetMinters {
        status: ResponseStatus,
    },

    // Other
    ChangeAdmin {
        status: ResponseStatus,
    },
    SetContractStatus {
        status: ResponseStatus,
    },

    // Permit
    RevokePermit {
        status: ResponseStatus,
    },

    // ftoken additions: 
    BatchReceiveNft {
        status: ResponseStatus,
    },
    Bid {
        status: ResponseStatus,
    },
    Stake {
        status: ResponseStatus,
    },
    Unstake {
        status: ResponseStatus,
    },
    VoteProposal {
        status: ResponseStatus,
    },
    FinalizeAuction {
        status: ResponseStatus,
    },
    RetrieveBid { 
        status: ResponseStatus,
    },
    ClaimProceeds { 
        status: ResponseStatus,
    },
    Propose {
        status: ResponseStatus,    
    },
    FinalizeExecuteProp {
        status: ResponseStatus,
    },
    RetrievePropStake {
        status: ResponseStatus,
    },
    VoteReservationPrice {
        status: ResponseStatus,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    TokenInfo {},
    TokenConfig {},
    ContractStatus {},
    ExchangeRate {},
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
        key: String,
    },
    Balance {
        address: HumanAddr,
        key: String,
    },
    TransferHistory {
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    TransactionHistory {
        address: HumanAddr,
        key: String,
        page: Option<u32>,
        page_size: u32,
    },
    Minters {},
    WithPermit {
        permit: Snip1155Permit,
        query: QueryWithPermit,
    },
    // ftoken additions:
    FtokenQuery(FtokenQuery),
    FtokenVkQuery {
        address: HumanAddr,
        key: String,
        query: FtokenAuthQuery,
    },
    // temporary for DEBUGGING. Must remove for final implementation
    // DebugQuery {  },
}

impl QueryMsg {
    pub fn get_validation_params(&self) -> (Vec<&HumanAddr>, ViewingKey) {
        match self {
            Self::Balance { address, key } => (vec![address], ViewingKey(key.clone())),
            Self::TransferHistory { address, key, .. } => (vec![address], ViewingKey(key.clone())),
            Self::TransactionHistory { address, key, .. } => {
                (vec![address], ViewingKey(key.clone()))
            }
            Self::Allowance {
                owner,
                spender,
                key,
                ..
            } => (vec![owner, spender], ViewingKey(key.clone())),
            // ftoken additions:
            Self::FtokenVkQuery { address, key, query: _  } => {
                (vec![address], ViewingKey(key.clone()))
            }
            _ => panic!("This query type does not require authentication"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
    },
    Balance {},
    TransferHistory {
        page: Option<u32>,
        page_size: u32,
    },
    TransactionHistory {
        page: Option<u32>,
        page_size: u32,
    },
    // ftoken additions:
    FtokenPermitQuery(FtokenAuthQuery),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    TokenInfo {
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: Option<Uint128>,
    },
    TokenConfig {
        public_total_supply: bool,
        deposit_enabled: bool,
        redeem_enabled: bool,
        mint_enabled: bool,
        burn_enabled: bool,
    },
    ContractStatus {
        status: ContractStatusLevel,
    },
    ExchangeRate {
        rate: Uint128,
        denom: String,
    },
    Allowance {
        spender: HumanAddr,
        owner: HumanAddr,
        allowance: Uint128,
        expiration: Option<u64>,
    },
    Balance {
        amount: Uint128,
    },
    TransferHistory {
        txs: Vec<Tx>,
        total: Option<u64>,
    },
    TransactionHistory {
        txs: Vec<RichTx>,
        total: Option<u64>,
    },
    ViewingKeyError {
        msg: String,
    },
    Minters {
        minters: Vec<HumanAddr>,
    },
    // ftoken additions:
    FtokenQueryAnswer (FtokenQueryAnswer),
    // temporary for DEBUGGING. Must remove for final implementation
    DebugQAnswer {
        ftokeninfo: FtokenInfo,
        // bids: Vec<BidInfo>,
        // won_bid: u32,
        ftkn_config: FtokenConf,
        next_prop_id: u32,
        nftviewingkey: ViewingKey,
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema)]
pub struct CreateViewingKeyResponse {
    pub key: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatusLevel {
    NormalRun,
    StopAllButRedeems,
    StopAll,
}

pub fn status_level_to_u8(status_level: ContractStatusLevel) -> u8 {
    match status_level {
        ContractStatusLevel::NormalRun => 0,
        ContractStatusLevel::StopAllButRedeems => 1,
        ContractStatusLevel::StopAll => 2,
    }
}

pub fn u8_to_status_level(status_level: u8) -> StdResult<ContractStatusLevel> {
    match status_level {
        0 => Ok(ContractStatusLevel::NormalRun),
        1 => Ok(ContractStatusLevel::StopAllButRedeems),
        2 => Ok(ContractStatusLevel::StopAll),
        _ => Err(StdError::generic_err("Invalid state level")),
    }
}

// Take a Vec<u8> and pad it up to a multiple of `block_size`, using spaces at the end.
pub fn space_pad(block_size: usize, message: &mut Vec<u8>) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_slice, StdResult};

    #[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum Something {
        Var { padding: Option<String> },
    }

    #[test]
    fn test_deserialization_of_missing_option_fields() -> StdResult<()> {
        let input = b"{ \"var\": {} }";
        let obj: Something = from_slice(input)?;
        assert_eq!(
            obj,
            Something::Var { padding: None },
            "unexpected value: {:?}",
            obj
        );
        Ok(())
    }
}
