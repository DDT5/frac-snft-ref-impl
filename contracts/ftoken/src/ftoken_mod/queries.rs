use cosmwasm_std::{
    to_binary, Storage, Api, Extern,
    Querier, QueryResult, 
};

use crate::{
    msg::{QueryAnswer},
    ftoken_mod::{
        state::{ftoken_contr_s_r, nft_vk_r, bid_id_r, bids_r}
    }
};

use super::state::allowed_bid_tokens_r;



/// temporary for DEBUGGING. Must remove for final implementation
pub fn debug_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> QueryResult {
    let ftokeninfo = ftoken_contr_s_r(&deps.storage).load()?;
    let next_bid_id = bid_id_r(&deps.storage).load()?;
    let mut bids = vec![];
    {
        let mut i = 0u32;
        while i < next_bid_id {
            bids.push(bids_r(&deps.storage).load(&i.to_le_bytes())?);
            i += 1;
        }
    }
    let allowed_bid_tokens = allowed_bid_tokens_r(&deps.storage).load()?;
    let nftviewingkey = nft_vk_r(&deps.storage).load()?;
    
    let resp = QueryAnswer::DebugQAnswer {
        ftokeninfo,
        bids,
        allowed_bid_tokens,
        next_bid_id,
        nftviewingkey,
    };
    to_binary(&(resp))

    // final implementation can use below response
    // to_binary("not available")
}