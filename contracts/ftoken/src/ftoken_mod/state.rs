use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Storage, Uint128, HumanAddr, StdResult, to_binary, 
};
use cosmwasm_storage::{
    PrefixedStorage, ReadonlyPrefixedStorage, 
    bucket, bucket_read, Bucket, ReadonlyBucket,
    singleton, singleton_read, ReadonlySingleton, Singleton,
};

use secret_toolkit::storage::{AppendStore, AppendStoreMut};

use crate::{
    viewing_key::ViewingKey
};
use fsnft_utils::{FtokenInfo, FtokenConf, AucConf};

use super::{
    msg::{
        Proposal
    }
};

// U256
use uint::{construct_uint};
construct_uint! { pub struct U256(4); }
construct_uint! { pub struct U192(3); }
construct_uint! { pub struct U384(6); }


pub const FTOKEN_CONTR_FTKN: &[u8] = b"ftkncontr_ftkn";
pub const FTKN_CONFIG: &[u8] = b"ftknconfig";
pub const FTKN_STAKE: &[u8] = b"ftknstake";
pub const ALLOWED_TOKENS: &[u8] = b"allowedtokens";
pub const PREFIX_BIDS: &[u8] = b"prefixbids";
pub const PROPS_STORE: &[u8] = b"propstore";
pub const PREFIX_UNDR_NFT: &[u8] = b"undrlynft";
pub const NFT_VIEW_KEY: &[u8] = b"nftviewkey";
pub const CURRENT_PROP_ID: &[u8] = b"currentproptid";
pub const VOTES_BUCKET: &[u8] = b"votesbucket";
pub const VOTES_TOTAL: &[u8] = b"votetotal";
pub const RESVPRICE_STORE: &[u8] = b"reservprice";
pub const AGGRESVPRICE_STORE: &[u8] = b"aggresvprice";
pub const AUCTION_INFO: &[u8] = b"auctioninfo";



/////////////////////////////////////////////////////////////////////////////////
// Buckets
/////////////////////////////////////////////////////////////////////////////////

/// staked ftokens
pub fn ftkn_stake_w<S: Storage>(storage: &mut S) -> Bucket<S, StakedTokens> {
    bucket(FTKN_STAKE, storage)
}
pub fn ftkn_stake_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, StakedTokens> {
    bucket_read(FTKN_STAKE, storage)
}

/// Links a bidder's HumanAddr (key) with the u32 pos (value here, key in the AppendStore storage).
/// Shares namespace the bid AppendStore storage, but should have no collision because this uses
/// HumanAddr as keys, and AppendStore uses pos: u32 as keys
fn bids_w<S: Storage>(storage: &mut S) -> Bucket<S, u32> {
    bucket(PREFIX_BIDS, storage)
}
fn bids_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, u32> {
    bucket_read(PREFIX_BIDS, storage)
}

/// Proposal storage: stores proposal information
pub fn props_w<S: Storage>(storage: &mut S) -> Bucket<S, PropInfo> {
    bucket(PROPS_STORE, storage)
}
pub fn props_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, PropInfo> {
    bucket_read(PROPS_STORE, storage)
}

/// proposal vote tally, which is the running cumulative tally
pub fn votes_total_w<S: Storage>(storage: &mut S) -> Bucket<S, VoteRegister> {
    bucket(VOTES_TOTAL, storage)
}
pub fn votes_total_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, VoteRegister> {
    bucket_read(VOTES_TOTAL, storage)
}

/// Reservation price votes for each address
pub fn resv_price_w<S: Storage>(storage: &mut S) -> Bucket<S, ResvVote> {
    bucket(RESVPRICE_STORE, storage)
}
pub fn resv_price_r<S: Storage>(storage: &S) -> ReadonlyBucket<S, ResvVote> {
    bucket_read(RESVPRICE_STORE, storage)
}


/////////////////////////////////////////////////////////////////////////////////
// Multi-level Buckets
/////////////////////////////////////////////////////////////////////////////////

/// Multilevel bucket to store proposal votes. Key intended to be [`bid_id`, HumanAddr]  
pub fn votes_w<S: Storage>(
    storage: &mut S,
    prop_id: u32
) -> Bucket<S, VoteRegister> {
    Bucket::multilevel(&[VOTES_BUCKET, &prop_id.to_le_bytes()], storage)
}
pub fn votes_r<S: Storage>(
    storage: &S,
    prop_id: u32
) -> ReadonlyBucket<S, VoteRegister> {
    ReadonlyBucket::multilevel(&[VOTES_BUCKET, &prop_id.to_le_bytes()], storage)
}


/////////////////////////////////////////////////////////////////////////////////
// Singletons
/////////////////////////////////////////////////////////////////////////////////


/// FtokenContr storage: stores information on this ftokens contract
pub fn ftoken_info_w<S: Storage>(storage: &mut S) -> Singleton<S, FtokenInfo> {
    singleton(storage, FTOKEN_CONTR_FTKN)
}
pub fn ftoken_info_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, FtokenInfo> {
    singleton_read( storage, FTOKEN_CONTR_FTKN)
}

/// config specifically for ftoken functionality
pub fn ftkn_config_w<S: Storage>(storage: &mut S) -> Singleton<S, FtokenConf> {
    singleton(storage, FTKN_CONFIG)
}
pub fn ftkn_config_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, FtokenConf> {
    singleton_read( storage, FTKN_CONFIG)
}

/// index the next proposal to be received 
pub fn prop_id_w<S: Storage>(storage: &mut S) -> Singleton<S, u32> {
    singleton(storage, CURRENT_PROP_ID)
}
pub fn prop_id_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, u32> {
    singleton_read( storage, CURRENT_PROP_ID)
}

/// information on auction
pub fn auction_info_w<S: Storage>(storage: &mut S) -> Singleton<S, AuctionInfo> {
    singleton(storage, AUCTION_INFO)
}
pub fn auction_info_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, AuctionInfo> {
    singleton_read( storage, AUCTION_INFO)
}

/// stores viewing key to query nft contract
pub fn nft_vk_w<S: Storage>(storage: &mut S) -> Singleton<S, ViewingKey> {
    singleton(storage, NFT_VIEW_KEY)
}

pub fn nft_vk_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ViewingKey> {
    singleton_read(storage, NFT_VIEW_KEY)
}

/// Aggregate reservation price 
pub fn agg_resv_price_w<S: Storage>(storage: &mut S) -> Singleton<S, ResvVote> {
    singleton(storage, AGGRESVPRICE_STORE)
}
pub fn agg_resv_price_r<S: Storage>(storage: &S) -> ReadonlySingleton<S, ResvVote> {
    singleton_read(storage, AGGRESVPRICE_STORE)
}

/////////////////////////////////////////////////////////////////////////////////
// Appendstore
/////////////////////////////////////////////////////////////////////////////////

// bids: Appendstore + bucket combo
// -----------------------------------------------------------------------------
pub fn add_bid<S: Storage>(
    store: &mut S,
    bid: &BidInfo,
) -> StdResult<()> {
    // appendstore: adds info with u32 key 
    let mut append_store = PrefixedStorage::new(PREFIX_BIDS, store);
    let mut append_store = AppendStoreMut::attach_or_create(&mut append_store)?;
    let len = append_store.len().clone(); // clone() should be unnecessary since u32 implement copy, but just to be safe 
    append_store.push(bid)?;

    // links user HumanAddr with pos: u32 key
    bids_w(store).save(
        to_binary(&bid.bidder)?.as_slice(), 
        &len
    )
}

pub fn set_bid<S: Storage>(
    store: &mut S,
    pos: u32,
    bid: &BidInfo,
) -> StdResult<()> {
    let mut store = PrefixedStorage::new(PREFIX_BIDS, store);
    let mut store = AppendStoreMut::attach_or_create(&mut store)?;
    store.set_at(pos, bid)
}

// pub fn set_bid_from_addr<S: Storage>(
//     store: &mut S,
//     addr: &HumanAddr,
//     bid: &BidInfo,
// ) -> StdResult<()> {
//     let pos = bids_r(store).load(&to_binary(&addr)?.as_slice())?;
//     set_bid(store, pos, bid)
// }

pub fn get_bid<S: Storage>(
    store: &S,
    pos: u32,
) -> StdResult<BidInfo> {
    let store = ReadonlyPrefixedStorage::new(PREFIX_BIDS, store);
    let store = AppendStore::<BidInfo, _, _>::attach(&store).unwrap()?;
    store.get_at(pos)
}

pub fn may_get_bid_from_addr<S: Storage>(
    store: &S,
    addr: &HumanAddr,
) -> StdResult<Option<(BidInfo, u32)>> {
    let pos_op = bids_r(store).may_load(&to_binary(&addr)?.as_slice())?;
    if let None = pos_op { return Ok(None) };
    let pos = pos_op.unwrap();
    let res_op = get_bid(store, pos).map(|bid_info| (bid_info, pos)).ok();
    Ok(res_op)
}

pub fn get_last_bid<S: Storage>(
    store: &S,
) -> StdResult<(BidInfo, u32)> {
    let store = ReadonlyPrefixedStorage::new(PREFIX_BIDS, store);
    let store = AppendStore::<BidInfo, _, _>::attach(&store).unwrap()?;
    let pos = store.len().saturating_sub(1);
    let last_bid = store.get_at(pos)?;
    Ok((last_bid, pos))
}

/// Returns all bids in reverse order and the number of bids.
/// If bids can be queried, bidders' addressed can be identified (even if addresses are removed
/// from the query result, if using side chain attacks)
pub fn get_bids<S: Storage>(
    store: &S,
    page: u32,
    page_size: u32,
) -> StdResult<(Vec<BidInfo>, u64)> {
    let store = ReadonlyPrefixedStorage::new(PREFIX_BIDS, store);
    let store = AppendStore::<BidInfo, _, _>::attach(&store);
    let store = if let Some(result) = store {
        result?
    } else { 
        return Ok((vec![BidInfo::default()], 0)) 
    };
    let bid_iter = store
        .iter()
        .rev()
        .skip((page * page_size) as _)
        .take(page_size as _);
        
    let bids: StdResult<Vec<BidInfo>> = bid_iter
        .map(|bid| bid)
        .collect();
    bids.map(|bids| (bids, store.len() as u64))
}

/////////////////////////////////////////////////////////////////////////////////
// Structs and enums
/////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakedTokens {
    pub amount: Uint128,
    pub unlock_height: u64,
}

impl Default for StakedTokens {
    fn default() -> Self {
        Self {
            amount: Uint128(0),
            unlock_height: 0u64,
        }
    }
}

/// Vote cast on proposals 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    /// If a proposal's veto votes are below the veto threshold, and quorum is met,
    /// the proposal will pass if yes votes > no votes
    Yes,
    No,
    /// If a certain threshold percentage of veto votes are made (determined by)
    /// the DAO configuration, the proposer will lose their staked ftokens
    Veto,
    /// Abstain votes count towards quorum (which need to be met for a proposal)
    /// to pass
    Abstain,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct VoteRegister {
    pub yes: Uint128,
    pub no: Uint128,
    pub veto: Uint128,
    pub abstain: Uint128,
}

// /// vote count, weighted by staked ftokens todo!() remove duplicate struct
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
// pub struct TotalVotes {
//     pub(crate) yes: Uint128,
//     pub(crate) no: Uint128,
//     pub(crate) veto: Uint128,
//     pub(crate) abstain: Uint128,
// }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum VoteResult {
    Won,
    Lost,
    LostWithVeto,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PropInfoTally {
    pub prop_info: PropInfo,
    pub vote_tally: VoteRegister,
}

/// proposal information as stored by ftoken contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PropInfo {
    /// proposal identifier
    pub prop_id: u32,
    /// address of proposer
    pub proposer: HumanAddr,
    /// proposal
    pub proposal: Proposal, 
    /// ftoken staked
    pub stake: Uint128,
    /// has the stake been withdrawn?
    pub stake_withdrawn: bool,
    /// outcome. If still in voting, `outcome` = `None`. If vote has been finalized, `outcome` = `VoteResult`
    pub outcome: Option<VoteResult>,
    // /// has the proposal been executed? A redundancy since outcome = Some(_) -> executed = true
    // pub executed: bool,
    /// block height where voting period ends. Final count tx can be called at this point forward
    pub end_height: u64,
}

// /// Proposal status
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// #[serde(rename_all = "snake_case")]
// pub enum PropStatus {
//     /// bid won, ftoken stake retrieved
//     PassedRetrieved,
//     /// won, but ftoken stake has not been retrieved
//     PassedNotRetrieved,
//     /// still active
//     Active, 
// }

// impl Default for PropStatus {
//     fn default() -> Self {
//         PropStatus::Active
//     }
// }


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionInfo {
    pub is_active: bool,
    pub end_height: u64,
    pub auc_config_snapshot: AucConf,
}

impl AuctionInfo {
    pub fn init() -> Self { 
        Self {
            is_active: false,
            end_height: 0u64,
            auc_config_snapshot: AucConf::default(),
        } 
    }
}

/// bid information as stored by ftoken contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct BidInfo {
    pub bidder: HumanAddr,
    /// amount denominated in the approved bid token
    pub amount: Uint128,
    /// did the bid win?
    pub winning_bid: bool,
    /// has the bidder retrieved the bid
    pub retrieved_bid: bool,
}

impl BidInfo {
    pub fn new(bidder: HumanAddr, amount: Uint128) -> Self {
        Self {
            bidder,
            amount,
            winning_bid: false,
            retrieved_bid: false,
        }
    }
}


/// Reservation price and stake stored in binary (serialized U192) 
/// representing a Uint128 with additional 19 decimal points
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ResvVote {
    pub stake: [u8; 24], 
    pub price: [u8; 24],
}

impl ResvVote {
    pub fn new(stake: Uint128, price: Uint128) -> ResvVote {
        let stake_192 = U192::from(stake.u128()).checked_mul(Self::precision()).unwrap(); 
        let price_192 = U192::from(price.u128()).checked_mul(Self::precision()).unwrap();

        Self { stake: Self::to_bin(stake_192), price: Self::to_bin(price_192) }
    }
    
    pub fn uint128_stake(&self) -> Uint128 {
        let stake_192 = U192::from_little_endian(self.stake.as_slice()); 
        Uint128(stake_192.checked_div(Self::precision()).unwrap().low_u128())
    }

    pub fn uint128_price(&self) -> Uint128 {
        let price_192 = U192::from_little_endian(self.price.as_slice()); 
        Uint128(price_192.checked_div(Self::precision()).unwrap().low_u128())
    }

    pub fn stake_mul_price(&self) -> U384 {
        let stake = U384::from_little_endian(self.stake.as_slice()); 
        let price = U384::from_little_endian(self.price.as_slice()); 
        stake.saturating_mul(price)
    }

    pub fn new_from_u384(stake: U384, price: U384) -> Self {
        let stake_384_bin = Self::to_bin_384(stake);
        let price_384_bin = Self::to_bin_384(price);

        let mut stake_192_bin = [0u8; 24];
        let mut price_192_bin = [0u8; 24];
        stake_192_bin.copy_from_slice(&stake_384_bin[..24]);
        price_192_bin.copy_from_slice(&price_384_bin[..24]);

        Self { stake: stake_192_bin, price: price_192_bin }
    }

    fn to_bin(num: U192) -> [u8; 24] {
        let mut num_bin = [0u8; 24];
        num.to_little_endian(&mut num_bin);
        num_bin
    }

    fn to_bin_384(num: U384) -> [u8; 48] {
        let mut num_bin = [0u8; 48];
        num.to_little_endian(&mut num_bin);
        num_bin
    }

    /// precision can be an arbirary number between 10^0 to 10^19
    fn precision() -> U192 { 
        U192::from(10u128.pow(19))
    }
}
