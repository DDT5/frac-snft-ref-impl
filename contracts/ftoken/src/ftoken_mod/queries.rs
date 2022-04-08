use cosmwasm_std::{
    to_binary, Storage, Api, Extern,
    Querier, QueryResult, 
};

use crate::{
    msg::{QueryAnswer},
    ftoken_mod::{
        state::{ftoken_info_r, nft_vk_r, prop_id_r, props_r}
    }
};

use super::state::{ftkn_config_r};



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