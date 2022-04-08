use std::{ops::{Add, Sub, },}; //Mul, Div 

use cosmwasm_std::{
    log, Api, Binary, Env, Extern, Uint128,
    HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage,
    from_binary, to_binary,
    CosmosMsg, WasmMsg, 
};

use crate::{
    contract::{try_transfer_impl},
    msg::{InitMsg, HandleAnswer, ResponseStatus::Success,},
    state::{Config, ReadonlyConfig, Balances, }, 
    // receiver::Snip20ReceiveMsg,
    ftoken_mod::{
        state::{
        prop_id_r, prop_id_w,
        nft_vk_w, nft_vk_r,
        ftoken_info_w, ftoken_info_r, props_w, props_r, add_bid, may_get_bid_from_addr,
        get_last_bid, set_bid,
        ftkn_stake_w, ftkn_stake_r, ftkn_config_w, ftkn_config_r,
        votes_w, votes_r, votes_total_w, votes_total_r, 
        agg_resv_price_w, agg_resv_price_r, resv_price_w, resv_price_r,
        auction_info_w, auction_info_r,
        PropInfo, StakedTokens, Vote, VoteRegister, TotalVotes, VoteResult,
        ResvVote, AuctionInfo, BidInfo,
        U256, 
        },
        msg::{InitRes, Proposal, AllowedNftMsg, S721HandleMsg, S721QueryMsg},
    }, 
    viewing_key::ViewingKey, 
};

use secret_toolkit::{
    utils::{Query, HandleCallback},
    snip721::{ViewerInfo, OwnerOfResponse}, 
    crypto::sha_256,
};
use fsnft_utils::{
    UndrNftInfo, FtokenInfo, FtokenConf, InterContrMsg,
    send_nft_msg,
};

use super::state::U384; 





/////////////////////////////////////////////////////////////////////////////////
// Entry-point functions
/////////////////////////////////////////////////////////////////////////////////

pub fn add_ftoken_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<Vec<CosmosMsg>> {
    // init bid_id = 0, and save allowed bid token
    prop_id_w(&mut deps.storage).save(&0u32)?;
    ftkn_config_w(&mut deps.storage).save(&msg.init_info.ftkn_conf)?;
    auction_info_w(&mut deps.storage).save(&AuctionInfo::init())?;
    agg_resv_price_w(&mut deps.storage).save(&ResvVote::new(
        Uint128(0),
        msg.init_info.init_resv_price,
    ))?;

    // InitResponse to fractionalizer contract to register this ftoken contract
    let reg_msg = InitRes::register_receive(msg.clone(), env.clone());
    let cosmos_msg_reg = reg_msg.to_cosmos_msg(
        msg.init_info.fract_hash,
        env.message.sender.clone(),
        None,
    )?; 

    // save ftoken_info
    let ftkn_instance =  match reg_msg {
        InitRes::ReceiveFtokenCallback { ftkn_instance } => ftkn_instance,
        _ => return Err(StdError::generic_err("ftoken contract failed to create register receive message")),
    };
    let ftoken_info = FtokenInfo { 
        instance: ftkn_instance,
        vault_active: true,
    };
    ftoken_info_w(&mut deps.storage).save(&ftoken_info)?;

    // set viewing key. Alternatively use query permits, but some older NFTs may not implement query permits
    // created prng_seed_hashed twice. Might save gas to create only once, but likely marginal
    // for greater security, use Secret Orcales (Scrt-RNG) to generate random numbers
    let prng_seed_hashed = sha_256(&msg.prng_seed.0);
    let vk = ViewingKey::new(&env, &prng_seed_hashed, &deps.api.canonical_address(&env.contract.address)?.as_slice());
    nft_vk_w(&mut deps.storage).save(&vk)?;
    let set_vk_msg = InitRes::SetViewingKey { key: vk.to_string(), padding: None };
    let cosmos_msg_setvk = set_vk_msg.to_cosmos_msg(
        msg.init_info.nft_info.nft_contr.code_hash,
        msg.init_info.nft_info.nft_contr.address,
        None,
    )?;

    // create cosmos msg vector
    let messages = vec![
        cosmos_msg_reg,
        cosmos_msg_setvk,
    ];

    Ok(messages)
}


/// Stake ftokens so ftoken holder can vote on reservation price or proposals.
/// If user has already voted, they will need to vote again after staking for the new
/// tokens to count towards their votes
pub fn try_stake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // transfer ftokens
    try_transfer_impl(
        deps, 
        &deps.api.canonical_address(&env.message.sender)?,
        &deps.api.canonical_address(&env.contract.address)?,
        amount,
        None,
        &env.block,
    )?;
    
    // save info after staking additional `amount` of ftokens
    // calculate new amount
    let ftkn_stake_op = ftkn_stake_r(&deps.storage).may_load(to_binary(&env.message.sender)?.as_slice())?;
    let ftkn_stake = match ftkn_stake_op {
        Some(i) => i,
        None => StakedTokens{ amount: Uint128(0), unlock_height: 0u64 }
    };
    let new_amount = Uint128(ftkn_stake.amount.u128().checked_add(amount.u128()).unwrap());

    // calculate new unlock height: max of current or min bond period
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    let new_unlock_height = ftkn_stake.unlock_height.max(env.block.height + ftkn_conf.min_ftkn_bond_prd);
    
    let staked_tokens = StakedTokens{
        amount: new_amount,
        unlock_height: new_unlock_height,
    };
    ftkn_stake_w(&mut deps.storage).save(to_binary(&env.message.sender)?.as_slice(), &staked_tokens)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Stake { status: Success })?),
    })
}

/// Unstake ftokens. 
/// Unlike staking, unstaking ftokens automatically changes the aggregate vote results on `reservation price`.
/// Note that no change in proposal vote is required, because ftokens are bonded at least until end of proposal
/// voting periods 
pub fn try_unstake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // check if ftokens are still bonded
    let ftkn_stake_op = ftkn_stake_r(&deps.storage).may_load(to_binary(&env.message.sender)?.as_slice())?;
    let ftkn_stake = match ftkn_stake_op {
        Some(i) => i,
        None => return Err(StdError::generic_err("this address has not staked ftokens before"))
    };
    if env.block.height < ftkn_stake.unlock_height {
        return Err(StdError::generic_err(format!("ftokens are still bonded. Will unbond at height {}", ftkn_stake.unlock_height)))
    }

    // transfer ftokens
    try_transfer_impl(
        deps, 
        &deps.api.canonical_address(&env.contract.address)?,
        &deps.api.canonical_address(&env.message.sender)?,
        amount,
        None,
        &env.block,
    )?;

    // save info after unstake `amount` of ftokens 
    let new_amount = ftkn_stake.amount.sub(amount)?;
    let staked_tokens = StakedTokens{
        amount: new_amount,
        unlock_height: ftkn_stake.unlock_height,
    };
    ftkn_stake_w(&mut deps.storage).save(to_binary(&env.message.sender)?.as_slice(), &staked_tokens)?;

    // change reservation votes changes to storage
    let curr_agg_resv = agg_resv_price_r(&deps.storage).load()?;
    let old_resv = resv_price_r(&deps.storage).load(&to_binary(&env.message.sender)?.as_slice())?;
    let new_resv = ResvVote::new(
        old_resv.uint128_stake().sub(amount)?,
        old_resv.uint128_price(),
    );
    resv_price_w(&mut deps.storage).save(&to_binary(&env.message.sender)?.as_slice(), &new_resv)?;
    let new_agg_resv = new_agg_resv_vote(&curr_agg_resv, &old_resv, &new_resv);
    agg_resv_price_w(&mut deps.storage).save(&new_agg_resv)?;
    
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Unstake { status: Success })?),
    })
}

/// handles ftoken holder votes on proposals
pub fn try_vote_proposal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    prop_id: u32,
    vote: Vote,
) -> StdResult<HandleResponse> {
    // check if proposal exists
    let prop_info_op = props_r(&deps.storage).may_load(&prop_id.to_le_bytes())?;
    let prop_info = match prop_info_op {
        Some(i) => i,
        None => return Err(StdError::generic_err("proposal id refers to a non-existent proposal")),
    };

    // check if proposal is still in voting period
    if prop_info.end_height < env.block.height {
        return Err(StdError::generic_err("proposal voting period has ended"))
    }

    // load staked ftokens of sender
    let sender = to_binary(&env.message.sender)?;
    let sender_u8 = sender.as_slice();
    let mut ftkn_stake = ftkn_stake_r(&deps.storage).load(sender_u8)?;

    // check if sender has voted before
    let vote_op = votes_r(&deps.storage, prop_id).may_load(to_binary(&env.message.sender)?.as_slice())?;
    let old_vote_reg = match vote_op {
        Some(i) => i,
        None => VoteRegister::default(),
    };

    // save new vote
    let mut new_vote_reg = VoteRegister::default();
    match vote {
        Vote::Yes => new_vote_reg.yes = ftkn_stake.amount,
        Vote::No => new_vote_reg.no = ftkn_stake.amount,
        Vote::Veto => new_vote_reg.veto = ftkn_stake.amount,
        Vote::Abstain => new_vote_reg.abstain = ftkn_stake.amount,
    };
    votes_w(&mut deps.storage, prop_id).save(sender_u8, &new_vote_reg)?;

    // update staked ftoken bonded period to max of i) current unlock height, ii) min bond period, iii) end of voting period for bid user just voted for 
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    ftkn_stake.unlock_height = ftkn_stake.unlock_height.max(
        env.block.height.checked_add(ftkn_conf.min_ftkn_bond_prd).unwrap().max(
            prop_info.end_height
        )
    );
    ftkn_stake_w(&mut deps.storage).save(sender_u8, &ftkn_stake)?;

    // adjust aggregate vote
    // load total votes
    let mut votes_total = votes_total_r(&deps.storage).load(&prop_id.to_le_bytes())?;

    // save net effect on (cumulative) vote total tally
    votes_total.yes = Uint128(votes_total.yes.u128().checked_add(new_vote_reg.yes.u128()).unwrap())
                        .sub(old_vote_reg.yes).unwrap();
    votes_total.no = Uint128(votes_total.no.u128().checked_add(new_vote_reg.no.u128()).unwrap())
                        .sub(old_vote_reg.no).unwrap();
    votes_total.veto = Uint128(votes_total.veto.u128().checked_add(new_vote_reg.veto.u128()).unwrap())
                        .sub(old_vote_reg.veto).unwrap();
    
    votes_total_w(&mut deps.storage).save(&prop_id.to_le_bytes(), &votes_total).unwrap();

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::VoteProposal { status: Success })?),
    })
}


/// # Arguments
/// * `stake` - the ftoken stake required when making a proposal. This can be retrieved
/// after the voting period, unless the result is no_with_veto
pub fn try_propose<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    proposal: Proposal,
    stake: Uint128,
) -> StdResult<HandleResponse> {
    // check that underlying NFT is still in vault -- not strictly necessary...
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    if ftkn_info.vault_active == false {
        return Err(StdError::generic_err("nft no longer in vault"))
    };

    // check that ftoken stake is adequate
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    if ftkn_conf.prop_conf.min_stake > stake {
        return Err(StdError::generic_err(format!(
            "need to stake at least {} ftokens",
            ftkn_conf.prop_conf.min_stake,
        )));
    }  

    match proposal {
        Proposal::MsgToNft { .. } => (),
        Proposal::ChangeConfig { ..} => (),
    };

    // load current prop_id
    let prop_id = prop_id_r(&deps.storage).load()?;
    
    let prop_info = PropInfo {
        prop_id,
        proposer: env.message.sender.clone(),
        proposal,
        stake,
        stake_withdrawn: false,
        outcome: None, 
        end_height: env.block.height + ftkn_conf.prop_conf.vote_period,
    };

    // transfer ftoken stake to contract
    try_transfer_impl(
        deps, 
        &deps.api.canonical_address(&env.message.sender)?,
        &deps.api.canonical_address(&env.contract.address)?,
        stake,
        None,
        &env.block,
    )?;

    // Note that prop_id: u32 implements copy, hence no borrowing issues here
    props_w(&mut deps.storage).save(&prop_id.to_le_bytes(), &prop_info)?;

    // initialize votes_total to 0
    votes_total_w(&mut deps.storage).save(&prop_id.to_le_bytes(), &TotalVotes::default())?;

    // add 1 to bid_id count
    prop_id_w(&mut deps.storage).save(&prop_id.add(1u32))?;

    Ok(HandleResponse{
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Propose { status: Success })?),
    })
}

/// function that executes when a bidder sends a bid
pub fn try_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // check that underlying NFT is still in vault
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    if ftkn_info.vault_active == false {
        return Err(StdError::generic_err("vault no longer active"))
    };

    // load SNIP20 token ContractInfo and auction status
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    let mut auc_info = auction_info_r(&deps.storage).load()?;
    
    // if auction is not yet live
    if auc_info.is_active == false {
        // error if bid is below reservation price
        let resv_info = agg_resv_price_r(&deps.storage).load()?;
        
        let config = Config::from_storage(&mut deps.storage);
        let curr_staked_bp = calc_pro_rata(
            resv_info.uint128_stake().u128(), 
            config.total_supply(), 
            10_000_u128
        )?;  

        if amount < resv_info.uint128_price() {
            return Err(StdError::generic_err(format!(
                "bid must be equal or greater than the reservation price of {}", resv_info.uint128_price()
            )))
        // error if vault is not yet `unlocked` ie: haven't reached threshold number of reservation votes
        } else if curr_staked_bp < ftkn_conf.auc_conf.unlock_threshold.u128() {
            return Err(StdError::generic_err(format!(
                "vault is not unlocked. Unlock threshold is {} basis points (unit of 1/10000); only {} basis points of ftokens have voted", 
                ftkn_conf.auc_conf.unlock_threshold, curr_staked_bp
            )));
        // if (not yet live) && (above reservation price) && (vault is unlocked) -> auction starts
        } else {
            auc_info.is_active = true;
            auc_info.end_height = env.block.height.saturating_add(ftkn_conf.auc_conf.auc_period);
            // save a snapshot of auc_conf
            auc_info.auc_config_snapshot = ftkn_conf.auc_conf;
            auction_info_w(&mut deps.storage).save(&auc_info)?;
        }
    // if auction is already live
    } else if auc_info.is_active == true {
        let (last_bid, _) = get_last_bid(&deps.storage)?;
        let min_bid = last_bid.amount
            .multiply_ratio(Uint128(auc_info.auc_config_snapshot.min_bid_inc.add(10_000) as u128), Uint128(10_000));
        // check that new bid is higher than the min_bid = highest_bid x min_bid_increment    
        if env.block.height > auc_info.end_height {
            return Err(StdError::generic_err("auction has closed"))
        } else if amount < min_bid {
            return Err(StdError::generic_err(format!(
                "bid needs to be at least {}", min_bid
            )))
        // check that auction has not closed (current block height has not passed end height)
        }
    } else { return Err(StdError::generic_err("this should not happen")) }

    // check that bidder has bid before -> if so, update_bid with incremental amount instead,
    let prev_bid_op = may_get_bid_from_addr(
        &deps.storage, 
        &env.message.sender
    )?;
    let transfer_amount = match prev_bid_op {
        Some((prev_bid, _)) => amount.sub(prev_bid.amount)?,
        None => amount,
    };

    // create `TransferFrom` msg to send to SNIP20 ("sSCRT") contract
    let message = snip20_transferfrom_msg(
        env.message.sender.clone(),
        env.contract.address,
        transfer_amount,
        auc_info.auc_config_snapshot.bid_token.address,
        auc_info.auc_config_snapshot.bid_token.code_hash
    )?;
    
    let messages = vec![message];

    // save new bid at the top of the storage stack. If updating bid, this should replace 
    // the old link between bidder's HumanAddr and the pos: u32 when the `bids_w` function is called  
    let bid_info = BidInfo::new(
        env.message.sender,
        amount,
    );
    add_bid(&mut deps.storage, &bid_info)?;

    Ok(HandleResponse{
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::Bid { status: Success })?),
    })
}

/// tx that anyone can call after a auction period is over
pub fn try_finalize_auction<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // check that vault is still active
    let mut ftkn_info = ftoken_info_r(&deps.storage).load()?;
    if ftkn_info.vault_active == false {
        return Err(StdError::generic_err("vault is no longer active"))
    };

    // check that auction period is over
    let auc_info = auction_info_r(&deps.storage).load()?;
    match auc_info.is_active {
        true => (),
        false => return Err(StdError::generic_err(
            "auction has not been triggered", 
        )),
    }
    if env.block.height < auc_info.end_height {
        return Err(StdError::generic_err(format!(
            "auction still in voting period. Ends on block {}", auc_info.end_height
        )))
    }

    // determine winning bidder
    let (mut winning_bid, pos) = get_last_bid(&deps.storage)?;
    
    // save winning_bid.winning_bid = true
    winning_bid.winning_bid = true;
    set_bid(&mut deps.storage, pos, &winning_bid)?;

    // transfer nft to winning bidder    
    let winner = winning_bid.bidder;
    let send_nft_msg = send_nft_msg(
        deps, 
        env, 
        ftkn_info.instance.init_nft_info.nft_contr.address.clone(), 
        ftkn_info.instance.init_nft_info.nft_contr.code_hash.clone(), 
        winner, 
        ftkn_info.instance.init_nft_info.token_id.clone(), 
        None,
    )?;

    // close vault: save state
    ftkn_info.vault_active = false;
    ftoken_info_w(&mut deps.storage).save(&ftkn_info)?;

    Ok(HandleResponse{
        messages: vec![send_nft_msg],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::FinalizeAuction { status: Success })?),
    })
}

pub fn try_finalize_vote_may_execute_proposal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    prop_id: u32,   
) -> StdResult<HandleResponse> {
    let mut prop_info = props_r(&deps.storage).load(&prop_id.to_le_bytes())?;

    // check if voting period is over
    if env.block.height < prop_info.end_height {
        return Err(StdError::generic_err("proposal still in voting"))
    }

    // finalize vote count
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    let vote_result = determine_vote_res(
        &mut deps.storage, 
        prop_id, 
        ftkn_conf.prop_conf.vote_quorum,
        ftkn_conf.prop_conf.veto_threshold,
    )?;
    
    // save vote result 
    prop_info.outcome = Some(vote_result);
    props_w(&mut deps.storage).save(&prop_id.to_be_bytes(), &prop_info)?;

    // control flow depending on vote result
    match prop_info.outcome {
        None => return Err(StdError::generic_err("this error message should not be reachable")),
        Some(vote_result) => match vote_result {
            VoteResult::Won => try_execute_proposal(deps, env, prop_info.proposal)?,
            VoteResult::Lost => (),
            VoteResult::LostWithVeto => (),
        }
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::FinalizeExecuteProp { status: Success })?),
    })

}

// pub fn try_retrieve_nft<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     bid_id: u32,   
// ) -> StdResult<HandleResponse> {
//     let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
//     // check that function caller is the bidder
//     if &bid_info.bidder != &env.message.sender {
//         return Err(StdError::generic_err(
//             "Cannot retrieve underlying NFT: You are not the bidder"
//         ));
//     }; 

//     // check that bid had won and nft is still in vault
//     if bid_info.status != BidStatus::WonInVault {
//         return Err(StdError::generic_err(
//             "Cannot retrieve underlying NFT: bid status is not `WonInVault`"
//         ));
//     }

//     // create CosmosMsg to NFT contract to transfer NFT to bid winner
//     let ftoken_info = ftoken_info_r(&deps.storage).load()?;
    
//     let sender = env.message.sender.clone();
//     let send_nft_msg = send_nft_msg(
//         deps, 
//         env, 
//         ftoken_info.instance.init_nft_info.nft_contr.address, 
//         ftoken_info.instance.init_nft_info.nft_contr.code_hash, 
//         sender, 
//         ftoken_info.instance.init_nft_info.token_id, 
//         None,
//     )?;

//     // change state to indicate that underlying NFT has been retrieved
//     bid_info.status = BidStatus::WonRetrieved;
//     bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

//     Ok(HandleResponse{
//         messages: vec![send_nft_msg],
//         log: vec![],
//         data: None,
//     })
// }

/// retrieve bid amount for bidders who lost
pub fn try_retrieve_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,  
) -> StdResult<HandleResponse> {
    // check that vault is no longer live
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    if let true = ftkn_info.vault_active {
        return Err(StdError::generic_err("vault is still active"))
    }

    // load user's bid info
    let bid_info_op = may_get_bid_from_addr(&deps.storage, &env.message.sender)?;
    if let None = bid_info_op {
        return Err(StdError::generic_err("you did not bid"))
    }
    let (mut bid_info, pos) = bid_info_op.unwrap();

    // check that bidder hasn't already retrieved bid
    if let true = bid_info.retrieved_bid {
        return Err(StdError::generic_err("you have already retrieved bid"))
    }

    // check that bidder isn't the winner
    if let true = bid_info.winning_bid {
        return Err(StdError::generic_err("you won the bid. You should have received the NFT"))
    }

    // create `Transfer` msg to send to SNIP20 ("sSCRT") contract, to transfer bid amount to losing bidder
    let auc_info = auction_info_r(&deps.storage).load()?;
    let message = snip20_transfer_msg(
        env.message.sender, 
        bid_info.amount, 
        auc_info.auc_config_snapshot.bid_token.address, 
        auc_info.auc_config_snapshot.bid_token.code_hash
    )?;

    let messages = vec![message];

    // change state to indicate that bid tokens have been retrieved
    bid_info.retrieved_bid = true;
    set_bid(&mut deps.storage, pos, &bid_info)?;

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::RetrieveBid { status: Success })?),
    })
}

/// retreive proposal stake
pub fn try_retrieve_prop_stake<S: Storage, A: Api, Q:Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    prop_id: u32,
) -> StdResult<HandleResponse> {
    // check that proposal status: vote result
    let mut prop_info = props_r(&deps.storage).load(&prop_id.to_le_bytes())?;
    match prop_info.outcome {
        None => return Err(StdError::generic_err("proposal has not been finalized and executed")),
        Some(ref vote_result) => match vote_result {
            VoteResult::Won => (),
            VoteResult::Lost => (),
            VoteResult::LostWithVeto => return Err(StdError::generic_err("stake lost because proposal result: no with veto")),
        },
    }
    // check that proposal status: stake withdrawn?
    match prop_info.stake_withdrawn {
        true => return Err(StdError::generic_err("Proposal stake has already been retrieved")),
        false => (),
    }

    // transfer ftoken stake back to user
    try_transfer_impl(
        deps, 
        &deps.api.canonical_address(&env.contract.address)?,
        &deps.api.canonical_address(&env.message.sender)?,
        prop_info.stake,
        None,
        &env.block,
    )?;

    // save new prop_info `stake_withdrawn` status
    prop_info.stake_withdrawn = true;
    props_w(&mut deps.storage).save(&prop_id.to_le_bytes(), &prop_info)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::VoteReservationPrice { status: Success })?),
    })
}

/// For ftoken holders to claim their pro-rata share of sale proceeds, after a bid has won 
pub fn try_claim_proceeds<S: Storage, A: Api, Q:Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    // check that vault has closed
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;
    if let true = ftkn_info.vault_active {
        return Err(StdError::generic_err("vault is still active"))
    }

    // storage: how much ftoken does sender have?
    let balances = Balances::from_storage(&mut deps.storage);
    let account_balance = balances.balance(&deps.api.canonical_address(&env.message.sender)?);

    // transfer ownership of all sender's ftokens to this contract
    // can change to burn mechanism: might be more robust to division inaccuracies esp for very small ftoken holders
    try_transfer_impl(
        deps, 
        &deps.api.canonical_address(&env.message.sender.clone())?,
        &deps.api.canonical_address(&env.contract.address.clone())?,
        Uint128(account_balance),
        None,
        &env.block,
    )?;

    // load winning bid amount
    let (winning_bid, _) = get_last_bid(&deps.storage)?;
    let sale_proceeds = winning_bid.amount;

    // calculate amount of bid (in SNIP20 tokens) to transfer to sender     
    let config = Config::from_storage(&mut deps.storage);
    let total_supply = config.total_supply();

    let pro_rata_proceeds = calc_pro_rata(account_balance, total_supply, sale_proceeds.u128())?;

    // can delete
    // // u128::MAX has 38 zeros. Even in the most extreme case, this shouldn't cause precision errors
    // // let precision = U256::from(10u8).pow(U256::from(39u8));
    // let precision = U256::MAX;

    // let tot_supply_u256 = U256::from(total_supply);
    // let sale_proceeds_u256 = U256::from(sale_proceeds.u128());
    // let acc_bal_u256 = U256::from(account_balance);
    // let acc_bal_u256_pres = acc_bal_u256.saturating_mul(precision);
    
    // // pro-rata proportion, in approx exp(39) precision
    // let pro_rata_percent_op = (acc_bal_u256_pres).checked_div(tot_supply_u256); 
    // let pro_rata_percent = match pro_rata_percent_op {
    //     None => return Err(StdError::generic_err("Total ftoken supply is zero...")),
    //     Some(i) => i,
    // };

    // // Note if sale_proceeds is u128::MAX, this still should not overflow as 2^256 = 2^128^2, and
    // // pro_rata_percent < u128::MAX. But note that U256::MAX != (u128::MAX)^2, although close, perhaps 
    // // due to the way it is implemented?
    // let pro_rata_proceeds = pro_rata_percent.saturating_mul(sale_proceeds_u256).checked_div(precision).unwrap().low_u128();
    // let pro_rata_proceeds = Uint128(pro_rata_proceeds);

    // create `Transfer` msg to send to SNIP20 ("sSCRT") contract, to transfer pro-rata proceeds to ftoken holder
    let ftoken_config = ftkn_config_r(&deps.storage).load()?;
    let message = snip20_transfer_msg(
        env.message.sender, 
        Uint128(pro_rata_proceeds), 
        ftoken_config.auc_conf.bid_token.address, 
        ftoken_config.auc_conf.bid_token.code_hash
    )?;

    let messages = vec![message];

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: Some(to_binary(&HandleAnswer::ClaimProceeds { status: Success })?),
    })
}

/// Implements voting a reservation price for the underlying NFT 
/// This tx increases bonded period of staked ftokens
pub fn try_vote_resv_price<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A ,Q>,
    env: Env,
    resv_price: Uint128,
) -> StdResult<HandleResponse> {
    // load existing aggregate reservation price
    let curr_agg = agg_resv_price_r(&deps.storage).load()?;
    
    // check that input reservation price is within bounds
    let ftkn_conf = ftkn_config_r(&deps.storage).load()?;
    let floor = curr_agg.uint128_price().multiply_ratio(Uint128(100), ftkn_conf.auc_conf.resv_boundary);
    let ceiling = curr_agg.uint128_price().multiply_ratio(ftkn_conf.auc_conf.resv_boundary, Uint128(100));
    if resv_price > ceiling || resv_price < floor {
        return Err(StdError::generic_err(
            format!("Reserve price out of bounds. Please set between {} and {}", floor, ceiling)
        ));
    }

    // load user's existing reservation price vote
    let curr_usr_resv_op = resv_price_r(&deps.storage).may_load(&to_binary(&env.message.sender)?.as_slice())?;
    let curr_usr_resv = match curr_usr_resv_op {
        Some(i) => i,
        None => ResvVote::default(),
    };
    
    // load staked ftokens of sender
    let sender = to_binary(&env.message.sender)?;
    let sender_u8 = sender.as_slice();
    let mut ftkn_stake = ftkn_stake_r(&deps.storage).load(sender_u8)?;

    // save reservation price
    let new_usr_resv = ResvVote::new(ftkn_stake.amount, resv_price);
    resv_price_w(&mut deps.storage).save(sender_u8, &new_usr_resv)?;

    // calculate and save new aggregate reservation price
    let new_agg_resv = new_agg_resv_vote(
        &curr_agg, 
        &curr_usr_resv, 
        &new_usr_resv
    );
    agg_resv_price_w(&mut deps.storage).save(&new_agg_resv)?;

    // update staked ftoken bonded period to max of i) current unlock height, ii) min bond period 
    ftkn_stake.unlock_height = ftkn_stake.unlock_height.max(
        env.block.height.checked_add(ftkn_conf.min_ftkn_bond_prd).unwrap()
    );
    ftkn_stake_w(&mut deps.storage).save(sender_u8, &ftkn_stake)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(to_binary(&HandleAnswer::VoteReservationPrice { status: Success })?),
    })
}



/////////////////////////////////////////////////////////////////////////////////
// Callback receiver functions
/////////////////////////////////////////////////////////////////////////////////

/// function to process `send` message from SNIP721 token (called by fractionalizer contract)
/// * `msg` - msg sent by NFT contract, called by fractionalizer contract. Type: `UndrNftInfo` 
pub fn try_batch_receive_nft<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    from: HumanAddr,
    token_ids: Vec<String>,
    msg: Option<Binary>,
) -> StdResult<HandleResponse> {
    // deserialize msg
    let undr_nft: UndrNftInfo = from_binary(&msg.unwrap())?;

    // temporary: log msg received
    let log_msg = vec![
        log("sender", &sender),
        log("from", &from),
        log("token_ids", format!("{:?}", &token_ids)),
        log("msg", format!("{:?}", &undr_nft))  
    ];

    // check if underlying NFT info matches
    let ftoken_info = ftoken_info_r(&deps.storage).load()?;
    if undr_nft != ftoken_info.instance.init_nft_info {
        return Err(StdError::generic_err("underling NFT info does not match"))
    }

    // verify sender is the expected SNIP721 contract
    let nft_contr = ftoken_info.instance.init_nft_info.nft_contr;
    if env.message.sender != nft_contr.address {
        return Err(StdError::generic_err("recieving `send` msg from incorrect NFT contract"))
    };

    // query to check if properly received underlying NFT    
    let query = S721QueryMsg::OwnerOf {
        token_id: undr_nft.token_id.clone(),
        viewer: Some(ViewerInfo {
            address: env.contract.address.clone(),
            viewing_key: nft_vk_r(&deps.storage).load()?.to_string(),
        }),
        include_expired: Some(false),
    };
    let query_response: OwnerOfResponse = query.query(
        &deps.querier,
        nft_contr.code_hash,
        nft_contr.address,
    )?;

    if query_response.owner_of.owner != Some(env.contract.address) {
        return Err(StdError::generic_err("nft not transferred to vault, reversing transaction"))
    } else if query_response.owner_of.approvals != vec![] {
        return Err(StdError::generic_err(
            "there are current approvals to transfer the nft, which is not allowed when nft is in the vault"
        ))
    }
    // optional using query permits
    // let permit = Permit {
    //     params: PermitParams {
    //         allowed_tokens: ,
    //         permit_name: ,
    //         chain_id: ,
    //         permissions: ,
    //     },
    //     signature: PermitSignature {
    //         pub_key: ,
    //         signature: ,
    //     },
    // };    
    
    Ok(HandleResponse {
        messages: vec![],
        log: log_msg,
        data: None,
    })
}


// /// SNIP20 sends back Snip20ReceiveMsg message. This function is called after:
// /// * (none)
// pub fn try_receive_snip20<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     env: Env,
//     snip20receivemsg: Snip20ReceiveMsg,   
// ) -> StdResult<HandleResponse> {
//     // load ftkn_conf
//     let ftkn_conf = ftkn_config_r(&deps.storage).load()?;

//     // security check: triggered by this contract (ie: the Receive{sender} account is this contract)
//     if snip20receivemsg.sender != env.contract.address {
//         return Err(StdError::generic_err(
//             "`HandleMsg::Receive` is not a public interface. The `sender` of the receive message must be this contract itself"
//         ));
//     }
//     // security check: comes from allowed (eg: sSCRT) token contract
//     if env.message.sender != ftkn_conf.auc_conf.bid_token.address { //<--- note, not auc_conf snapshot
//         return Err(StdError::generic_err(
//             "Enter token contract address of an allowed bid token"
//         ));
//     }
    
//     Ok(HandleResponse::default())
// }


/////////////////////////////////////////////////////////////////////////////////
// Private functions
/////////////////////////////////////////////////////////////////////////////////

/// function to generate `Transfer` cosmos_msg to send to SNIP20 token contract
/// # Arguments
/// * `recipient` - token transfer to this address
/// * `amount` - amount of tokens to send in smallest denomination
/// * `contract_addr` - the address of the SNIP20 contract
/// * `callback_code_hash` - the code hash of the SNIP20 contract
fn snip20_transfer_msg(
    recipient: HumanAddr,
    amount: Uint128,
    contract_addr: HumanAddr,
    callback_code_hash: String,
) -> StdResult<CosmosMsg> {
    let message = to_binary(&InterContrMsg::Transfer{
        recipient,
        amount,
        memo: None,
        padding: None,
    })?;
    let cosmos_message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        callback_code_hash,
        msg: message,
        send: vec![],
    });
    
    Ok(cosmos_message)
}


/// function to generate `TransferFrom` cosmos_msg to send to SNIP20 token contract
/// # Arguments
/// * `owner` - token transfers from this address
/// * `recipient` - token transfer to this address
/// * `amount` - amount of tokens to send in smallest denomination
/// * `contract_addr` - the address of the SNIP20 contract
/// * `callback_code_hash` - the code hash of the SNIP20 contract
fn snip20_transferfrom_msg(
    owner: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
    contract_addr: HumanAddr,
    callback_code_hash: String,
) -> StdResult<CosmosMsg> {
    let message = to_binary(&InterContrMsg::TransferFrom{
        owner,
        recipient,
        amount,
        memo: None,
        padding: None,
    })?;
    let cosmos_message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        callback_code_hash,
        msg: message,
        send: vec![],
    });
    
    Ok(cosmos_message)
}

// /// function to generate `SendFrom` cosmos_msg to send to SNIP20 token contract
// /// # Arguments
// /// * `owner` - token transfers from this address
// /// * `recipient` - token transfer to this address
// /// * `recipient_code_hash` - optional hash
// /// * `amount` - amount of tokens to send in smallest denomination
// /// * `msg_to_recipient` - optional cosmos message to send to recipient
// /// * `contract_addr` - the address of the SNIP20 contract
// /// * `callback_code_hash` - the code hash of the SNIP20 contract
// fn snip20_sendfrom_msg(
//     owner: HumanAddr,
//     recipient: HumanAddr,
//     recipient_code_hash: Option<String>,
//     amount: Uint128,
//     msg_to_recipient: Option<Binary>,
//     contract_addr: HumanAddr,
//     callback_code_hash: String,
// ) -> StdResult<CosmosMsg> {
//     let message = to_binary(&InterContrMsg::SendFrom{
//         owner,
//         recipient,
//         recipient_code_hash,
//         amount,
//         msg: msg_to_recipient,
//         memo: None,
//         padding: None,
//     })?;
//     let cosmos_message = CosmosMsg::Wasm(WasmMsg::Execute {
//         contract_addr,
//         callback_code_hash,
//         msg: message,
//         send: vec![],
//     });
    
//     Ok(cosmos_message)
// }

// /// function to generate `Send` cosmos_msg to send to SNIP20 token contract
// /// `SendFrom` appears to require approval even if owner `SendFrom` its own tokens 
// /// * `recipient` - token transfer to this address
// /// * `recipient_code_hash` - optional hash
// /// * `amount` - amount of tokens to send in smallest denomination
// /// * `msg_to_recipient` - optional cosmos message to send to recipient
// /// * `contract_addr` - the address of the SNIP20 contract
// /// * `callback_code_hash` - the code hash of the SNIP20 contract
// fn snip20_send_msg(
//     recipient: HumanAddr,
//     recipient_code_hash: Option<String>,
//     amount: Uint128,
//     msg_to_recipient: Option<Binary>,
//     contract_addr: HumanAddr,
//     callback_code_hash: String,
// ) -> StdResult<CosmosMsg> {
//     let message = to_binary(&InterContrMsg::Send{
//         recipient,
//         recipient_code_hash,
//         amount,
//         msg: msg_to_recipient,
//         memo: None,
//         padding: None,
//     })?;
//     let cosmos_message = CosmosMsg::Wasm(WasmMsg::Execute {
//         contract_addr,
//         callback_code_hash,
//         msg: message,
//         send: vec![],
//     });
    
//     Ok(cosmos_message)
// }

// /// Changes a bid status. Also saves bid_id with `won_bid_id_w` if it wins
// /// * `bid_id` - the bid index
// /// * `status: BidStatus` - the status to change to
// fn change_bid_status<S: Storage>(
//     storage: &mut S,
//     // _env: Env,
//     bid_id: u32,
//     status: BidStatus,
// ) -> StdResult<HandleResponse> {

//     // save winning bid
//     match status {
//         BidStatus::WonRetrieved => {
//             won_bid_id_w(storage).save(&bid_id)?;
//             let mut ftoken_info = ftoken_info_r(storage).load()?;
//             ftoken_info.vault_active = false;
//             ftoken_info_w(storage).save(&ftoken_info)?;
//         },
//         BidStatus::WonInVault => {
//             won_bid_id_w(storage).save(&bid_id)?;
//             let mut ftoken_info = ftoken_info_r(storage).load()?;
//             ftoken_info.vault_active = false;
//             ftoken_info_w(storage).save(&ftoken_info)?;
//         },
//         BidStatus::Active => (),
//         BidStatus::LostInTreasury => (),
//         BidStatus::LostRetrieved => (),
//     }

//     // change bid status
//     let mut bid_info = bids_r(storage).load(&bid_id.to_le_bytes())?;
//     bid_info.status = status;
//     bids_w(storage).save(&bid_id.to_le_bytes(), &bid_info)?;

//     Ok(HandleResponse::default())
// }

/// determines result of vote
fn determine_vote_res<S: Storage>(
    // deps: &mut Extern<S, A, Q>,
    storage: &mut S,
    prop_id: u32,
    quorum: Uint128,
    veto_threshold: Uint128,
) -> StdResult<VoteResult> {
    // final vote tally
    let vote_tally = votes_total_r(storage).load(&prop_id.to_le_bytes())?;

    // load configs
    let config = ReadonlyConfig::from_storage(storage);

    // if vote >= veto threshold -> LostWithVeto
    let veto_proportion = (vote_tally.veto)
        .multiply_ratio(Uint128(10_000), Uint128(config.total_supply()));
    if veto_proportion >= veto_threshold {
        return Ok(VoteResult::LostWithVeto)
    }

    // if `yes` + `no` + `veto` + `abstain` < quorum -> Lost
    let vote_proportion = (vote_tally.yes + vote_tally.no + vote_tally.veto + vote_tally.abstain)
        .multiply_ratio(Uint128(10_000), Uint128(config.total_supply()));
    if vote_proportion < quorum {
        return Ok(VoteResult::Lost)
    }

    // if no >= yes -> Lost 
    if vote_tally.no >= vote_tally.yes {
        return Ok(VoteResult::Lost)
    }

    // if yes > no -> Won
    if vote_tally.yes > vote_tally.no {
        return Ok(VoteResult::Won)
    }

    // else, something strange happened.... should not be reachable
    return Err(StdError::generic_err("unable to determine vote result"))
}

/// private function: execute proposal if won
fn try_execute_proposal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    proposal: Proposal,
) -> StdResult<()> {
    match proposal {
        Proposal::MsgToNft { msg } => {
            try_send_msg_to_nft(
                &deps.storage,
                msg,
            )?;
        },
        Proposal::ChangeConfig { config } => {
            try_change_config(&mut deps.storage, config)?;
        },
    }

    return Ok(())
}

/// sends message to underlying NFT
fn try_send_msg_to_nft<S: Storage>(
    storage: &S,
    msg: AllowedNftMsg,   
) -> StdResult<HandleResponse> {
    let ftkn_info = ftoken_info_r(storage).load()?;
    let token_id = ftkn_info.instance.init_nft_info.token_id; 

    // cosmos_msg to be sent to SNIP721
    let message = match msg {
        AllowedNftMsg::SetMetadata { public_metadata, private_metadata 
        } => S721HandleMsg::SetMetadata {
            token_id,
            public_metadata,
            private_metadata,
            padding: None,
        },
        AllowedNftMsg::Reveal {  } => S721HandleMsg::Reveal { 
            token_id, 
            padding: None, 
        },
        AllowedNftMsg::MakeOwnershipPrivate {  } => S721HandleMsg::MakeOwnershipPrivate { padding: None },
        AllowedNftMsg::SetGlobalApproval { view_owner, view_private_metadata, expires 
        } => S721HandleMsg::SetGlobalApproval { 
            token_id: Some(token_id), 
            view_owner, 
            view_private_metadata, 
            expires, 
            padding: None,
        },
        AllowedNftMsg::SetWhitelistedApproval { address, view_owner, view_private_metadata, expires 
        } => S721HandleMsg::SetWhitelistedApproval { 
            address, 
            token_id: Some(token_id), 
            view_owner, 
            view_private_metadata, 
            // cannot have transfer permissions while fractionalized
            transfer: None, 
            expires, 
            padding: None,
        },
    };

    // create cosmos_msg binary
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: ftkn_info.instance.init_nft_info.nft_contr.address,
        callback_code_hash: ftkn_info.instance.init_nft_info.nft_contr.code_hash,
        msg: to_binary(&message)?,
        send: vec![],
    });
    
    Ok(HandleResponse {
        messages: vec![cosmos_msg],
        log: vec![],
        data: None,
    })
}

/// private function: changes ftoken contract config
/// note that config does not change the config of a live auction
fn try_change_config<S: Storage>(
    storage: &mut S,
    config: FtokenConf,
) -> StdResult<HandleResponse> {
    ftkn_config_w(storage).save(&config)?;
    Ok(HandleResponse::default())
}

// function to calculate new aggregate reservation prices -> ResvVote{stake, price}
fn new_agg_resv_vote(curr_agg: &ResvVote, old: &ResvVote, new: &ResvVote) -> ResvVote {
    let old_agg_stake = U384::from_little_endian(curr_agg.stake.as_slice());
    let new_agg_stake = old_agg_stake
        .saturating_add(U384::from_little_endian(new.stake.as_slice()))
        .saturating_sub(U384::from_little_endian(old.stake.as_slice()));
    // if new_agg_stake is zero, leave the current agg price,(which will determine the boundaries
    // of any new reservation price votes   
    let mut new_agg_price = U384::zero();
    if new_agg_stake <= U384::zero() {
        new_agg_price = U384::from_little_endian(curr_agg.price.as_slice())
    } else if new_agg_stake > U384::zero() {
        new_agg_price = curr_agg.stake_mul_price()
            .saturating_add(new.stake_mul_price())
            .saturating_sub(old.stake_mul_price())
            .checked_div(new_agg_stake).unwrap_or_else(|| panic!("new_agg_stake zero, which should not happen"))
    }
    ResvVote::new_from_u384(
        new_agg_stake, 
        new_agg_price,
    )
}

fn calc_pro_rata(
    num: u128,
    denom: u128,
    normalized_to: u128,
) -> StdResult<u128> {
    // u128::MAX has 38 zeros. Even in the most extreme case, this shouldn't cause precision errors
    let precision = U256::from(u128::MAX);

    let num_u256 = U256::from(num);
    let denom_u256 = U256::from(denom);
    let normalizedto_u256 = U256::from(normalized_to);

    // Note 2^256 = 2^128^2. Multiply first, then divide, so does not saturate
    let pro_rata_normalized = num_u256
        .saturating_mul(precision)
        .checked_div(denom_u256).unwrap()    
        .saturating_mul(normalizedto_u256)
        .checked_div(precision).unwrap()
        .low_u128();

    Ok(pro_rata_normalized)
}

/////////////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////////////
// Note comprehensive unit and integration tests are located in the `int_test` package

#[cfg(test)]
mod tests {
    use crate::{
        ftoken_mod::{
            state::*,
        }
    };
    use cosmwasm_std::{
        // testing::*,
        Uint128
    };

    use std::{
        ops::{Sub, Mul }, // Mul, Add, Div, DivAssign
        // convert::TryInto
    };

    use super::*;

    #[test]
    fn new_agg_resv_vote_works() {
        // net effect on reservation price votes
        let user0 = ResvVote::new(
            Uint128(2906469),
            Uint128(417),
        );
        let user1_0 = ResvVote::new(
            // precision of 10^19 as defined in status.rs
            Uint128(10_u128.pow(19_u32)),
            Uint128(242),
        );
        let user1_1 = ResvVote::new( 
            Uint128(2),
            Uint128(8750000),
        );
        let user1_2 = ResvVote::new(
            Uint128(4689854),
            Uint128(634),
        );
        let exp_agg_0 = ResvVote::new(
            Uint128(10_000_000_000_002_906_469),
            Uint128(242),
        );
        let exp_agg_1 = ResvVote::new(
            Uint128(2906471),
            Uint128(423),
        );
        let exp_agg_2 = ResvVote::new(
            Uint128(7596323),
            Uint128(550),
        );

        let agg_00 = new_agg_resv_vote(
            &ResvVote::default(), 
            &ResvVote::default(), 
            &user0
        );
        let agg_0 = new_agg_resv_vote(
            &agg_00, 
            &ResvVote::default(), 
            &user1_0
        );
        let agg_1 = new_agg_resv_vote(
            &agg_0, 
            &user1_0, 
            &user1_1
        );
        let agg_2 = new_agg_resv_vote(
            &agg_1, 
            &user1_1,
            &user1_2
        );
        
        // check any precision drift in agg value due to rounding?
        assert_eq!(agg_0.uint128_stake(), exp_agg_0.uint128_stake());
        assert_eq!(agg_0.uint128_price(), exp_agg_0.uint128_price());

        assert_eq!(agg_1.uint128_stake(), exp_agg_1.uint128_stake());
        assert_eq!(agg_1.uint128_price(), exp_agg_1.uint128_price());

        assert_eq!(agg_2.uint128_stake(), exp_agg_2.uint128_stake());
        assert_eq!(agg_2.uint128_price(), exp_agg_2.uint128_price());
    }

    #[test]
    fn test_pro_rata_calc() {
        // extreme number tests. shouldn't ever round up
        // when denom < norm:
        // test at u128::MAX
        let max = u128::MAX;
        let mut res = calc_pro_rata(12u128, max.checked_div(100u128).unwrap(), max).unwrap();
        assert_eq!(res, 1200u128); // precisely correct

        // test at close to max of u128: precisely calculated number should be 1200; round down is OK, round up not OK
        res = calc_pro_rata(12u128,10u128.pow(36), 10u128.pow(38)).unwrap();
        assert_eq!(res, 1199u128); 
        
        // test precision: round down not 10^N, but only 1 (ie at the smallest denomination of sscrt)
        res = calc_pro_rata(12u128,10_u128.pow(18).mul(12u128), 10u128.pow(38)).unwrap();
        assert_eq!(res, 100_000_000_000_000_000_000_u128.sub(1u128)); 

        // when denom > norm: test at max
        res = calc_pro_rata(12_000_000u128, max, max.checked_div(10_000u128).unwrap()).unwrap();
        assert_eq!(res, 1199u128); // rounds down by 1
    }


// Temporary debugging tests
// -----------------------------------------------------------------------------

    #[test]
    fn test_arithmetic_temp() {
        // U256 test --------------------------------------
        // let precision = U256::from(10u8).pow(U256::from(39u8));
        // dbg!(&precision);
        // dbg!(&precision.low_u128());
        // let max = U256::from(u128::MAX);
        // let mut max2 = max.mul(max);
        // dbg!(&max2);
        // max2 += 2u128.into();
        // dbg!(&max2);
        // let mut max256 = U256::from(0u8);
        // max256 = max256.overflowing_sub(U256::from(1u8)).0;
        // dbg!(&max256);
        // dbg!(&max256-&max2);
        
        // U[X] tests
        let mut max128 = Uint128(0);
        max128 = Uint128(max128.u128().wrapping_sub(1u128)); 
        println!("{:?}", max128); // 3.4028E+38
        let mut max192 = U192::from(0u8);
        max192 = max192.overflowing_sub(U192::from(1u8)).0;
        println!("{:?}", max192); // 6.2771E+57
        let mut max384 = U384::from(0u8);
        max384 = max384.overflowing_sub(U384::from(1u8)).0;
        println!("{:?}", max384); // 3.9402E+115
        let squared = max192.overflowing_mul(max192);
        println!("{:?}", squared);
        // println!("{:?}", max192.as_u128()); // panics
        println!("{:?}", max192.low_u128());
        println!("{:?}", max192.saturating_sub(U192::from(1_234_567u128)).low_u128());

        // div is euclid
        let a = U256::from(10u128);
        let b = U256::from(3u128);
        let c = a.checked_div(b).unwrap();
        println!("{}", c);
        assert_eq!(c.as_u128(), 3u128);

        // outliers ----------------------------------------
        // let mut a: Vec<i128> = vec![61,8,12,10,16];
        // a.sort();
        // println!("{:?}", a);
        // let mut total = 0i128;
        // for i in a.iter() {
        //     total += i;
        // }
        // let mean = total.div_euclid(a.len().try_into().unwrap()); 
        // println!("{}", &mean);

        // // average absolute deviation
        // let mut aad = 0128;
        
        // for i in a.iter() {
        //     aad += i.sub(mean);
        // }
        // aad = aad.div_euclid(a.len().try_into().unwrap());
        // println!("{}", aad);
    }  

    #[test]
    fn test_resvvote_bin_temp() {
        let precision = U384::from(10u128.pow(19));
        let resv_vote = ResvVote::new(Uint128(50), Uint128(100));
        let new_from_u384 = ResvVote::new_from_u384(
            U384::from(200_u128),
            U384::from(400_u128),
        );
        println!("binary: {:?}, \nUint128 price: {:?}, \nUint128 stake: {:?}, \nStakeMulPrice: {:?}, \nnew_from_u384: {:?}",   
        resv_vote, 
        resv_vote.uint128_stake(),
        resv_vote.uint128_price(), 
        resv_vote.stake_mul_price().checked_div(precision).unwrap().checked_div(precision),
        new_from_u384
    );
    }
}