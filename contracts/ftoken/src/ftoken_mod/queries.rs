use cosmwasm_std::{
    to_binary, Storage, Api, Extern,
    Querier, QueryResult, 
};

use crate::{
    msg::{QueryAnswer},
    ftoken_mod::{
        state::{ftoken_contr_s_r, nft_vk_r}
    }
};



/// temporary for DEBUGGING. Must remove for final implementation
pub fn debug_query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> QueryResult {
    let ftokeninfo = ftoken_contr_s_r(&deps.storage).load()?;
    let nftviewingkey = nft_vk_r(&deps.storage).load()?;
    
    let resp = QueryAnswer::DebugQAnswer {
        ftokeninfo,
        nftviewingkey,
    };
    to_binary(&(resp))

    // final implementation can use below response
    // to_binary("not available")
}