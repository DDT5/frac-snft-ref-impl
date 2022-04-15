use cosmwasm_std::{
    to_binary, Storage, Api, Extern,
    Querier, QueryResult, HumanAddr, StdError, StdResult, 
};
use secret_toolkit::{
    snip721::ViewerInfo, 
    utils::Query
};

use crate::{
    msg::{QueryAnswer}, 
    state::{ReadonlyBalances, ReadonlyConfig},
};

use super::{
    handles::{calc_pro_rata},
    state::{
        ftoken_info_r, nft_vk_r, prop_id_r, props_r, ftkn_config_r, agg_resv_price_r,
        get_bids, ftkn_stake_r, resv_price_r, votes_total_r, 
        PropInfoTally, votes_r, may_get_bid_from_addr,
    }, 
    msg::{FtokenQuery, FtokenAuthQuery, FtokenQueryAnswer, S721QueryMsg, 
        PrivateMetadataResponse, NftDossierResponse}, 
    ft_permit::{Permit, Permission}
};

/////////////////////////////////////////////////////////////////////////////////
// Match arms for queries
/////////////////////////////////////////////////////////////////////////////////

pub fn ftoken_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    query: FtokenQuery,
) -> QueryResult {
    match query {
        FtokenQuery::FtokenInfo {  } => query_ftoken_info(&deps.storage),
        FtokenQuery::FtokenConfig {  } => query_ftoken_config(&deps.storage),
        FtokenQuery::AuctionConfig {  } => query_auction_config(&deps.storage),
        FtokenQuery::ProposalConfig {  } => query_proposal_config(&deps.storage),
        FtokenQuery::ReservationPrice {  } => query_reservation_config(&deps.storage),
        FtokenQuery::ProposalList {  } => query_proposal_list(&deps.storage),
        // enabling this reduces the privacy of bidders. Blockchain analysis or side chain attacks
        // can easily reveal address of bidders
        FtokenQuery::BidList { page, page_size } => query_bid_list(&deps.storage, page, page_size),
    }
}


pub fn ftoken_permit_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    permit: Permit,
    account: &HumanAddr, 
    query: FtokenAuthQuery,
) -> QueryResult {
    match query {
        FtokenAuthQuery::NftPrivateMetadata {  } => {
            if !permit.check_permission(&Permission::NftPrivateMetadata) {
                return Err(StdError::generic_err(format!(
                    "No permission to query underlying NFT private metadata, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_nft_priv_metadata(&deps, account)
        },
        FtokenAuthQuery::NftDossier {  } => {
            if !permit.check_permission(&Permission::NftDossier) {
                return Err(StdError::generic_err(format!(
                    "No permission to query underlying NFT Dossier, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_nft_dossier(&deps, account)
        },
        FtokenAuthQuery::StakedTokens {  } => {
            if !permit.check_permission(&Permission::StakedTokens) {
                return Err(StdError::generic_err(format!(
                    "No permission to query staked tokens, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_staked_tokens(&deps.storage, account)
        },
        FtokenAuthQuery::ReservationPriceVote {  } => {
            if !permit.check_permission(&Permission::ReservationPriceVote) {
                return Err(StdError::generic_err(format!(
                    "No permission to query reservation price votes, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_reservation_price_vote(&deps.storage, account)
        },
        FtokenAuthQuery::ProposalVotes { prop_id } => {
            if !permit.check_permission(&Permission::ProposalVotes) {
                return Err(StdError::generic_err(format!(
                    "No permission to query proposal votes, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_proposal_votes(&deps.storage, account, prop_id)
        },
        FtokenAuthQuery::Bid {  } => {
            if !permit.check_permission(&Permission::ProposalVotes) {
                return Err(StdError::generic_err(format!(
                    "No permission to query proposal votes, got permissions {:?}",
                    permit.params.permissions
                )));
            }

            query_bid(&deps.storage, account)
        },
    }
}


pub fn ftoken_viewing_keys_queries<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    account: &HumanAddr, 
    query: FtokenAuthQuery,
) -> QueryResult {
    match query {
        FtokenAuthQuery::NftPrivateMetadata {  } => query_nft_priv_metadata(&deps, account),
        FtokenAuthQuery::NftDossier {  } => query_nft_dossier(&deps, account),
        FtokenAuthQuery::StakedTokens {  } => query_staked_tokens(&deps.storage, account),
        FtokenAuthQuery::ReservationPriceVote {  } => query_reservation_price_vote(&deps.storage, account),
        FtokenAuthQuery::ProposalVotes { prop_id } => query_proposal_votes(&deps.storage, account, prop_id),
        FtokenAuthQuery::Bid {  } => query_bid(&deps.storage, account),
    }
}

/// temporary for DEBUGGING. Must remove for final implementation
pub fn debug_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> QueryResult {
    let ftokeninfo = ftoken_info_r(&deps.storage).load()?;
    let next_prop_id = prop_id_r(&deps.storage).load()?;
    // let mut bids = vec![];
    let mut props = vec![];
    {
        let mut i = 0u32;
        while i < next_prop_id {
            // let bid = bids_r(&deps.storage).may_load(&i.to_le_bytes())?;
            // if let Some(b) = bid {bids.push(b)};
            
            let prop = props_r(&deps.storage).may_load(&i.to_le_bytes())?;
            if let Some(p) = prop {props.push(p)};
            i += 1;
        }
    }
    // let won_bid = won_bid_id_r(&deps.storage).load()?;
    let ftkn_config = ftkn_config_r(&deps.storage).load()?;
    let nftviewingkey = nft_vk_r(&deps.storage).load()?;
    
    let resp = QueryAnswer::DebugQAnswer {
        ftokeninfo,
        // bids,
        // won_bid,
        ftkn_config,
        next_prop_id,
        nftviewingkey,
    };
    to_binary(&(resp))

    // final implementation can use below response
    // to_binary("not available")
}

/////////////////////////////////////////////////////////////////////////////////
// Functions for queries: public queries
/////////////////////////////////////////////////////////////////////////////////

fn query_ftoken_info<S: Storage>(
    storage: &S,
) -> QueryResult {
    let ftkn_info = ftoken_info_r(storage).load()?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::FtokenInfo { 
        ftkn_info 
    }))
}

fn query_ftoken_config<S: Storage>(
    storage: &S,
) -> QueryResult {
    let ftkn_conf = ftkn_config_r(storage).load()?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::FtokenConfig {
        ftkn_conf,
    }))
}

fn query_auction_config<S: Storage>(
    storage: &S,
) -> QueryResult {
    let ftkn_conf = ftkn_config_r(storage).load()?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::AuctionConfig { 
        auc_conf: ftkn_conf.auc_conf,
    }))
}

fn query_proposal_config<S: Storage>(
    storage: &S,
) -> QueryResult {
    let ftkn_conf = ftkn_config_r(storage).load()?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::ProposalConfig {
        prop_conf: ftkn_conf.prop_conf,
    }))
}

fn query_reservation_config<S: Storage>(
    storage: &S,
) -> QueryResult {
    let agg_resv = agg_resv_price_r(storage).load()?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::ReservationPrice {
        ftokens_voted: agg_resv.uint128_stake(),
        reservation_price: agg_resv.uint128_price(),
    }))
}

fn query_proposal_list<S: Storage>(
    storage: &S,
) -> QueryResult {
    let next_prop_id = prop_id_r(storage).load()?;

    let mut prop_info_tally_list = vec![];
    {
        let mut i = 0u32;
        while i < next_prop_id {
            let prop_info = props_r(storage).may_load(&i.to_le_bytes())?;
            let vote_tally = votes_total_r(storage).may_load(&i.to_le_bytes())?;

            if let Some(p) = prop_info {
                if let Some(v) = vote_tally {
                    prop_info_tally_list.push(
                        PropInfoTally {
                            prop_info: p,
                            vote_tally: v,
                        }
                    )
                }
            };

            i += 1;
        }
    }

    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::ProposalList( 
        prop_info_tally_list,
    )))
}

fn query_bid_list<S: Storage>(
    storage: &S,
    page: u32,
    page_size: u32,
) -> QueryResult {
    let (bids, total_bids) = get_bids(storage, page, page_size)?;
    let bid_amounts = bids.iter().map(
        | bid_info | bid_info.amount
    ). collect();
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::BidList {
        bid_amounts,
        total_bids,
    }))
}


/////////////////////////////////////////////////////////////////////////////////
// Functions for queries: authenticated queries
/////////////////////////////////////////////////////////////////////////////////

fn query_nft_priv_metadata<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    account: &HumanAddr,
) -> QueryResult {
    check_ftoken_query_threshold(&deps, &account)?;

    // query private metadata
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    let query = S721QueryMsg::PrivateMetadata {
        token_id: ftkn_info.instance.init_nft_info.token_id,
        viewer: Some(ViewerInfo {
            address: ftkn_info.instance.ftoken_contr.address,
            viewing_key: nft_vk_r(&deps.storage).load()?.to_string(),
        }),
    };
    let query_response: PrivateMetadataResponse = query.query(
        &deps.querier,
        ftkn_info.instance.init_nft_info.nft_contr.code_hash,
        ftkn_info.instance.init_nft_info.nft_contr.address,
    )?;
    
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::NftPrivateMetadata(
        query_response)
    ))
}

fn query_nft_dossier<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    account: &HumanAddr,
) -> QueryResult {
    check_ftoken_query_threshold(&deps, &account)?;

    // query Nft Dossier
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    let query = S721QueryMsg::NftDossier { 
        token_id: ftkn_info.instance.init_nft_info.token_id,
        viewer: Some(ViewerInfo {
            address: ftkn_info.instance.ftoken_contr.address,
            viewing_key: nft_vk_r(&deps.storage).load()?.to_string(),
        }),
        include_expired: Some(true),
    };
    let query_response: NftDossierResponse = query.query(
        &deps.querier,
        ftkn_info.instance.init_nft_info.nft_contr.code_hash,
        ftkn_info.instance.init_nft_info.nft_contr.address,
    )?;
    
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::NftDossier(
        query_response)
    ))
}

fn query_staked_tokens<S:Storage>(
    storage: &S,
    account: &HumanAddr,
) -> QueryResult {
    let staked_ftkns = ftkn_stake_r(storage).load(&to_binary(&account)?.as_slice())?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::StakedTokens(
        staked_ftkns
    )))
}

fn query_reservation_price_vote<S: Storage>(
    storage: &S,
    account: &HumanAddr,
) -> QueryResult {
    let resv_vote = resv_price_r(storage).load(&to_binary(&account)?.as_slice())?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::ReservationPriceVote(
        resv_vote
    )))
}

fn query_proposal_votes<S: Storage>(
    storage: &S,
    account: &HumanAddr,
    prop_id: u32,
) -> QueryResult {
    let vote = votes_r(storage, prop_id).load(&to_binary(&account)?.as_slice())?;
    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::ProposalVotes(
        vote
    )))
} 

fn query_bid<S: Storage>(
    storage: &S,
    account: &HumanAddr,
) -> QueryResult {
    let bid_info_op = may_get_bid_from_addr(storage, &account)?;
    if let None = bid_info_op { return Err(StdError::generic_err(
        "you do not have a live bid"
    ))}
    let (bid, _) = bid_info_op.unwrap();

    to_binary(&QueryAnswer::FtokenQueryAnswer(FtokenQueryAnswer::Bid(
        bid
    )))
} 

/////////////////////////////////////////////////////////////////////////////////
// Private functions
/////////////////////////////////////////////////////////////////////////////////

fn check_ftoken_query_threshold<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>, 
    account: &HumanAddr,
) -> StdResult<()> {
    // calculate required ftokens to view private metadata: threshold
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    let threshold = ftkn_conf.priv_metadata_view_threshold;

    // calculate required ftokens to view private metadata: total supply
    let config = ReadonlyConfig::from_storage(&deps.storage);
    let total_supply = config.total_supply();
    // required tokens = threshold basis points / 10_000 * total_supply
    let req_tokens = calc_pro_rata(threshold.into(), 10_000, total_supply)?;

    // user's tokens = staked ftokens + owned ftokens
    let usr_stake = ftkn_stake_r(&deps.storage).load(&to_binary(&account)?.as_slice()).unwrap_or_default(); 
    let ftkn_bal = ReadonlyBalances::from_storage(&deps.storage).account_amount(
        &deps.api.canonical_address(&account).unwrap()
    );    
    // check balance
    if usr_stake.amount.u128().saturating_add(ftkn_bal) < req_tokens { return Err(StdError::generic_err(format!(
        "you need at least {} ftokens to view private metadata",
        req_tokens
    )))} else { return Ok(()) }
}
