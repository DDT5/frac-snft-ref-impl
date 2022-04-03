use std::ops::{Mul, Add, Sub};
use uint::{construct_uint};

use cosmwasm_std::{
    log, Api, Binary, Env, Extern, Uint128,
    HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage,
    from_binary, to_binary,
    CosmosMsg, WasmMsg, debug_print, 
};

use crate::{
    contract::{try_transfer_impl},
    msg::{InitMsg},
    state::{Balances, Config, ReadonlyConfig}, 
    receiver::Snip20ReceiveMsg,
    ftoken_mod::{
        state::{
        nft_vk_w, nft_vk_r,
        ftoken_info_w, ftoken_info_r, bids_w, bids_r,
        won_bid_id_w, won_bid_id_r, ftkn_stake_w, ftkn_stake_r, ftkn_config_w, ftkn_config_r,
        votes_w, votes_r, Vote, votes_total_w, votes_total_r,
        },
        msg::{InitRes},
    }, 
    viewing_key::ViewingKey, 
};

use secret_toolkit::{
    utils::{Query, HandleCallback},
    snip721::{ViewerInfo, OwnerOfResponse}, 
    crypto::sha_256,
};
use fsnft_utils::{UndrNftInfo, S721QueryMsg, BidsInfo, BidStatus, InterContrMsg, FtokenInfo, send_nft_msg};

use super::state::{bid_id_r, bid_id_w, StakedTokens, VoteRegister, TotalVotes};


/////////////////////////////////////////////////////////////////////////////////
// Entry-point functions
/////////////////////////////////////////////////////////////////////////////////

pub fn add_ftoken_init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<Vec<CosmosMsg>> {
    // init bid_id = 0, and save allowed bid token
    bid_id_w(&mut deps.storage).save(&0u32)?;
    ftkn_config_w(&mut deps.storage).save(&msg.init_info.ftkn_conf)?;
    // allowed_bid_tokens_w(&mut deps.storage).save(&msg.init_info.bid_token)?;

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
        in_vault: true,
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


/// function that executes when a bidder sends a bid
pub fn try_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // check underlying NFT is still in vault
    let ftkn_info = ftoken_info_r(&deps.storage).load()?;

    // load SNIP20 token ContractInfo
    let ftoken_config = ftkn_config_r(&deps.storage).load()?;

    // create `SendFrom` msg to send to SNIP20 ("sSCRT") contract
    let message = snip20_sendfrom_msg(
        env.message.sender,
        env.contract.address,
        Some(env.contract_code_hash),
        amount,
        Some(to_binary(&ftkn_info)?),
        ftoken_config.bid_conf.bid_token.address,
        ftoken_config.bid_conf.bid_token.code_hash
    )?;
    
    let messages = vec![message];

    Ok(HandleResponse{
        messages,
        log: vec![],
        data: None,
    })
}

/// SNIP20 sends back Snip20ReceiveMsg message. This function is called after:
/// * try_bid
pub fn try_receive_snip20<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    snip20receivemsg: Snip20ReceiveMsg,   
) -> StdResult<HandleResponse> {
    // load ftkn_config
    let ftkn_config = ftkn_config_r(&deps.storage).load()?;

    // security check: triggered by this contract (ie: the Receive{sender} account is this contract)
    if snip20receivemsg.sender != env.contract.address {
        return Err(StdError::generic_err(
            "`HandleMsg::Receive` is not a public interface. The `sender` of the receive message must be this contract itself"
        ));
    }
    // security check: comes from allowed (eg: sSCRT) token contract
    if env.message.sender != ftkn_config.bid_conf.bid_token.address {
        return Err(StdError::generic_err(
            "Enter token contract address of an allowed bid token"
        ));
    }

    // check which ftoken contract the bid refers to
    let ftoken_info: FtokenInfo = from_binary(&snip20receivemsg.msg.unwrap())?;
    
    // check ftoken contract holds a biddable (still-fractionalized) NFT // add this info into FtokenInfo struct
    if ftoken_info.in_vault != true {
        return Err(StdError::generic_err(
            "NFT is no longer in the vault"
        ));
    }

    // query to doublecheck that correct  bid token and amount are indeed received (extra security, as a SNIP20 compliant contract 
    // should not allow a tx to happen if tokens fail to be sent)
    // Can: Check bid amount matches quantity of received tokens: snip20receivemsg.amount = <bid amount>
    // todo!() second priority

    // acknowledge receipt and save into storage
    let bid_id = bid_id_r(&deps.storage).load()?;
    let bid_info = BidsInfo {
        bid_id,
        // the bidder is the address that sent the bid tokens
        bidder: snip20receivemsg.from,
        amount: snip20receivemsg.amount,
        status: BidStatus::Active,
        end_height: env.block.height + ftkn_config.bid_conf.bid_period, 
    };
    // Note that bid_id: u32 implements copy, hence no borrowing issues here
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    // initialize votes_total to 0
    votes_total_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &TotalVotes::default())?;

    // add 1 to bid_id count
    bid_id_w(&mut deps.storage).save(&bid_id.add(1u32))?;
    
    Ok(HandleResponse::default())
}

/// stake ftokens
pub fn try_stake<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    // calculate new unlock height 
    let ftkn_config = ftkn_config_r(&deps.storage).load()?;
    let new_unlock_height = env.block.height + ftkn_config.bid_conf.min_ftkn_bond_prd;

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
    let ftkn_stake_op = ftkn_stake_r(&deps.storage).may_load(to_binary(&env.message.sender)?.as_slice())?;
    let ftkn_stake = match ftkn_stake_op {
        Some(i) => i,
        None => StakedTokens{ amount: Uint128(0), unlock_height: 0u64 }
    };
    let new_amount = Uint128(ftkn_stake.amount.u128().checked_add(amount.u128()).unwrap());
    let staked_tokens = StakedTokens{
        amount: new_amount,
        unlock_height: new_unlock_height,
    };
    ftkn_stake_w(&mut deps.storage).save(to_binary(&env.message.sender)?.as_slice(), &staked_tokens)?;

    Ok(HandleResponse::default())
}

/// unstake ftokens
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

    Ok(HandleResponse::default())
}

/// handles ftoken holder votes on bids
pub fn try_vote<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bid_id: u32,
    vote: Vote,
) -> StdResult<HandleResponse> {
    // check if bid exists
    let bid_info_op = bids_r(&deps.storage).may_load(&bid_id.to_le_bytes())?;
    let bid_info = match bid_info_op {
        Some(i) => i,
        None => return Err(StdError::generic_err("bid_id refers to a non-existent bid")),
    };

    // check if bid is still in voting period
    if bid_info.end_height < env.block.height {
        return Err(StdError::generic_err("bid voting period has ended"))
    }

    // load total votes
    let mut votes_total = votes_total_r(&deps.storage).load(&bid_id.to_le_bytes())?;

    // load staked ftokens of sender
    let sender = to_binary(&env.message.sender)?;
    let sender_u8 = sender.as_slice();
    let ftkn_stake = ftkn_stake_r(&deps.storage).load(sender_u8)?;

    // check if sender has voted before
    let vote_op = votes_r(&deps.storage, bid_id).may_load(to_binary(&env.message.sender)?.as_slice())?;
    let old_vote_reg = match vote_op {
        Some(i) => i,
        None => VoteRegister::default(),
    };

    // save new vote
    let mut vote_reg = VoteRegister::default();
    match vote {
        Vote::Yes => vote_reg.yes = ftkn_stake.amount,
        Vote::No => vote_reg.no = ftkn_stake.amount,
    };
    votes_w(&mut deps.storage, bid_id).save(sender_u8, &vote_reg)?;

    // save net effect on (cumulative) vote total tally
    votes_total.yes = Uint128(votes_total.yes.u128().checked_add(vote_reg.yes.u128()).unwrap())
                        .sub(old_vote_reg.yes).unwrap();
    votes_total.no = Uint128(votes_total.no.u128().checked_add(vote_reg.no.u128()).unwrap())
                        .sub(old_vote_reg.no).unwrap();
    votes_total_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &votes_total).unwrap();

    Ok(HandleResponse::default())
}

/// tx that anyone can call after a `bid_id` voting period is over, to perform the final vote count
pub fn try_finalize_vote_count<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bid_id: u32,
) -> StdResult<HandleResponse> {
    // check that no bid has won yet
    let ftoken_info = ftoken_info_r(&deps.storage).load()?;
    if ftoken_info.in_vault == false {
        return Err(StdError::generic_err("nft no longer in vault"))
    };

    // check that bid_id voting period is over
    let bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
    if bid_info.end_height > env.block.height {
        return Err(StdError::generic_err("bid has not reached end of voting period"))
    }

    // final vote tally
    let vote_tally = votes_total_r(&deps.storage).load(&bid_id.to_le_bytes())?;

    // load configs
    let config = ReadonlyConfig::from_storage(&deps.storage);
    let ftoken_conf = ftkn_config_r(&deps.storage).load()?;

    // check if quorum is reached. If not, change status of bid to LostInTreasury
    let vote_proportion = (vote_tally.yes + vote_tally.no).multiply_ratio(Uint128(10_000), Uint128(config.total_supply()));
    debug_print!("Vote proportion is: {}, Vote quorum is: {}", vote_proportion, ftoken_conf.bid_conf.bid_vote_quorum);
    if ftoken_conf.bid_conf.bid_vote_quorum > vote_proportion {
        change_bid_status(&mut deps.storage, bid_id, BidStatus::LostInTreasury)?;
        debug_print!("Changed bid_status to `LostInTreasury`");
        return Ok(HandleResponse::default())
    }

    // if yes > no, change bid status to WonInVault 
    if vote_tally.yes > vote_tally.no {
        change_bid_status(&mut deps.storage, bid_id, BidStatus::WonInVault)?;
        debug_print!("Changed bid_status to `WonInVault`");
        return Ok(HandleResponse::default())
    }
        
    // if no > yes, change bid status to LostInTreasury 
    if vote_tally.no > vote_tally.yes {
        change_bid_status(&mut deps.storage, bid_id, BidStatus::LostInTreasury)?;
        debug_print!("Changed bid_status to `LostInTreasury`");
        return Ok(HandleResponse::default())
    }

    return Err(StdError::generic_err("unable to determine vote result"))
}

pub fn try_retrieve_nft<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bid_id: u32,   
) -> StdResult<HandleResponse> {
    let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
    // check that function caller is the bidder
    if &bid_info.bidder != &env.message.sender {
        return Err(StdError::generic_err(
            "Cannot retrieve underlying NFT: You are not the bidder"
        ));
    }; 

    // check that bid had won and nft is still in vault
    if bid_info.status != BidStatus::WonInVault {
        return Err(StdError::generic_err(
            "Cannot retrieve underlying NFT: bid status is not `WonInVault`"
        ));
    }

    // create CosmosMsg to NFT contract to transfer NFT to bid winner
    let ftoken_info = ftoken_info_r(&deps.storage).load()?;
    
    let sender = env.message.sender.clone();
    let send_nft_msg = send_nft_msg(
        deps, 
        env, 
        ftoken_info.instance.init_nft_info.nft_contr.address, 
        ftoken_info.instance.init_nft_info.nft_contr.code_hash, 
        sender, 
        ftoken_info.instance.init_nft_info.token_id, 
        None,
    )?;

    // change state to indicate that underlying NFT has been retrieved
    bid_info.status = BidStatus::WonRetrieved;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    Ok(HandleResponse{
        messages: vec![send_nft_msg],
        log: vec![],
        data: None,
    })
}

pub fn try_retrieve_bid<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    bid_id: u32,   
) -> StdResult<HandleResponse> {
    let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
    // check that function caller is the bidder
    if &bid_info.bidder != &env.message.sender {
        return Err(StdError::generic_err(
            "Cannot retrieve bid tokens: You are not the bidder"
        ));
    }; 

    // check that bid had lost and bid amount is still in treasury
    if bid_info.status != BidStatus::LostInTreasury {
        return Err(StdError::generic_err(
            "Cannot retrieve bid tokens: bid status is not `LostInTreasury`"
        ));
    }
    
    // create `Send` msg to send to SNIP20 ("sSCRT") contract, to transfer bid back to bidder who lost
    // load SNIP20 token ContractInfo
    let ftoken_config = ftkn_config_r(&deps.storage).load()?;
    let amount = bid_info.amount;
    
    let message = snip20_send_msg(
        env.message.sender, 
        None, 
        amount, 
        None, 
        ftoken_config.bid_conf.bid_token.address, 
        ftoken_config.bid_conf.bid_token.code_hash,
    )?;

    let messages = vec![message];

    // change state to indicate that bid tokens have been retrieved
    bid_info.status = BidStatus::LostRetrieved;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    Ok(HandleResponse{
        messages,
        log: vec![],
        data: None,
    })
}


/// For ftoken holders to claim their pro-rata share of sale proceeds, after a bid has won 
pub fn try_claim_proceeds<S: Storage, A: Api, Q:Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let won_bid_id = won_bid_id_r(&deps.storage).load()?;
    let bid_info = bids_r(&deps.storage).load(&won_bid_id.to_le_bytes())?;
    // check if BidStatus is `WonRetrieved` or `WonInVault`
    if bid_info.status != BidStatus::WonRetrieved && bid_info.status != BidStatus::WonInVault {
        return Err(StdError::generic_err(
            "Cannot claim proceeds"
        ));
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

    // calculate amount of bid (in SNIP20 tokens) to transfer to sender 
    let bid_size = bid_info.amount;
    
    let config = Config::from_storage(&mut deps.storage);
    let total_supply = config.total_supply();

    construct_uint! {
        pub struct U256(4);
    }
    // u128::MAX has 38 zeros. Even in the most extreme case, this shouldn't cause precision errors
    let precision = U256::from(10u8).pow(U256::from(39u8));

    let tot_supply_u256 = U256::from(total_supply);
    let bid_size_u256 = U256::from(bid_size.u128());
    let acc_bal_u256 = U256::from(account_balance);
    let acc_bal_u256_pres = acc_bal_u256.checked_mul(precision).unwrap();
    
    // pro-rata proportion, in approx exp(38) precision
    let pro_rata_percent_op = (acc_bal_u256_pres).checked_div(tot_supply_u256); 
    let pro_rata_percent = match pro_rata_percent_op {
        None => return Err(StdError::generic_err("Total ftoken supply is zero... How did this happen?")),
        Some(i) => i,
    };

    // Note if bid_size is u128::MAX, this still should not overflow as 2^256 = 2^128^2, and
    // pro_rata_percent < u128::MAX. But note that U256::MAX > (u128::MAX)^2, perhaps due to the way it is 
    // implemented?
    let pro_rata_proceeds = pro_rata_percent.mul(bid_size_u256).checked_div(precision).unwrap().low_u128();
    let pro_rata_proceeds = Uint128(pro_rata_proceeds);

    // create `SendFrom` msg to send to SNIP20 ("sSCRT") contract, to transfer pro-rata proceeds to ftoken holder
    let ftoken_config = ftkn_config_r(&deps.storage).load()?;
    let message = snip20_send_msg(
        env.message.sender, 
        None, 
        pro_rata_proceeds, 
        None, 
        ftoken_config.bid_conf.bid_token.address, 
        ftoken_config.bid_conf.bid_token.code_hash
    )?;

    let messages = vec![message];

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None,
    })
}


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

/////////////////////////////////////////////////////////////////////////////////
// Private functions
/////////////////////////////////////////////////////////////////////////////////

/// function to generate `SendFrom` cosmos_msg to send to SNIP20 token contract
/// # Arguments
/// * `owner` - token transfers from this address
/// * `recipient` - token transfer to this address
/// * `recipient_code_hash` - optional hash
/// * `amount` - amount of tokens to send in smallest denomination
/// * `msg_to_recipient` - optional cosmos message to send to recipient
/// * `contract_addr` - the address of the SNIP20 contract
/// * `callback_code_hash` - the code hash of the SNIP20 contract
fn snip20_sendfrom_msg(
    owner: HumanAddr,
    recipient: HumanAddr,
    recipient_code_hash: Option<String>,
    amount: Uint128,
    msg_to_recipient: Option<Binary>,
    contract_addr: HumanAddr,
    callback_code_hash: String,
) -> StdResult<CosmosMsg> {
    let message = to_binary(&InterContrMsg::SendFrom{
        owner,
        recipient,
        recipient_code_hash,
        amount,
        msg: msg_to_recipient,
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

/// function to generate `Send` cosmos_msg to send to SNIP20 token contract
/// `SendFrom` appears to require approval even if owner `SendFrom` its own tokens 
/// * `recipient` - token transfer to this address
/// * `recipient_code_hash` - optional hash
/// * `amount` - amount of tokens to send in smallest denomination
/// * `msg_to_recipient` - optional cosmos message to send to recipient
/// * `contract_addr` - the address of the SNIP20 contract
/// * `callback_code_hash` - the code hash of the SNIP20 contract
fn snip20_send_msg(
    recipient: HumanAddr,
    recipient_code_hash: Option<String>,
    amount: Uint128,
    msg_to_recipient: Option<Binary>,
    contract_addr: HumanAddr,
    callback_code_hash: String,
) -> StdResult<CosmosMsg> {
    let message = to_binary(&InterContrMsg::Send{
        recipient,
        recipient_code_hash,
        amount,
        msg: msg_to_recipient,
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

/// Changes a bid status. Also saves bid_id with `won_bid_id_w` if it wins
/// * `bid_id` - the bid index
/// * `status: BidStatus` - the status to change to
fn change_bid_status<S: Storage>(
    storage: &mut S,
    // _env: Env,
    bid_id: u32,
    status: BidStatus,
) -> StdResult<HandleResponse> {

    // save winning bid
    match status {
        BidStatus::WonRetrieved => {
            won_bid_id_w(storage).save(&bid_id)?;
            let mut ftoken_info = ftoken_info_r(storage).load()?;
            ftoken_info.in_vault = false;
            ftoken_info_w(storage).save(&ftoken_info)?;
        },
        BidStatus::WonInVault => {
            won_bid_id_w(storage).save(&bid_id)?;
            let mut ftoken_info = ftoken_info_r(storage).load()?;
            ftoken_info.in_vault = false;
            ftoken_info_w(storage).save(&ftoken_info)?;
        },
        BidStatus::Active => (),
        BidStatus::LostInTreasury => (),
        BidStatus::LostRetrieved => (),
    }

    // change bid status
    let mut bid_info = bids_r(storage).load(&bid_id.to_le_bytes())?;
    bid_info.status = status;
    bids_w(storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    Ok(HandleResponse::default())
}

// /// TEMPORARY function to be eventually deleted
// /// Allows user to directly change bid status with JSON messages
// /// * `bid_idx` - the bid index
// /// * `status` - the status to change to. A `u8` number that corresponds to the desired status 
// pub fn try_change_bid_status<S: Storage, A: Api, Q: Querier>(
//     deps: &mut Extern<S, A, Q>,
//     _env: Env,
//     bid_id: u32,
//     status_idx: u8,
//     winning_bid: Option<u32>,
// ) -> StdResult<HandleResponse> {
//     let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
//     let status = match status_idx {
//         0 => BidStatus::WonRetrieved,
//         1 => BidStatus::WonInVault,
//         2 => BidStatus::Active,
//         3 => BidStatus::LostInTreasury,
//         4 => BidStatus::LostRetrieved,  
//         _ => BidStatus::Active, // temp... ie: if invalid index, just keep it at BidStatus::Active, which is the default
//     };
//     bid_info.status = status;
//     bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

//     // save winning bid
//     if let Some(won_bid_id) = winning_bid {
//         won_bid_id_w(&mut deps.storage).save(&won_bid_id)?;
//     }

//     Ok(HandleResponse::default())
// }

/////////////////////////////////////////////////////////////////////////////////
// Temporary debugging tests. 
/////////////////////////////////////////////////////////////////////////////////
// Note the unit and integration tests are located elsewhere

#[cfg(test)]
mod tests {
    use crate::{
        // contract::*, 
        // msg::*,
        ftoken_mod::{
            state::*,
        }
    };
    use cosmwasm_std::{testing::*, to_binary, Uint128};


    use std::ops::{Sub, Mul};

    // U256
    use uint::{construct_uint};
    
    construct_uint! {
        pub struct U256(4);
    }

    #[test]
    fn temp_test() {
        let precision = U256::from(10u8).pow(U256::from(39u8));
        dbg!(&precision);
        dbg!(&precision.low_u128());
        let max = U256::from(u128::MAX);
        let mut max2 = max.mul(max);
        dbg!(&max2);
        max2 += 2u128.into();
        dbg!(&max2);
        let mut max256 = U256::from(0u8);
        max256 = max256.overflowing_sub(U256::from(1u8)).0;
        dbg!(&max256);
        dbg!(&max256-&max2);
    }

    #[test]
    fn test_try_vote_temp() {
        let env = mock_env("voter", &[]);
        let mut deps = mock_dependencies(20, &[]);
        let bid_id = 0u32;
        let vote = Vote::No;
        // optional: voted before
        let old_vote = VoteRegister { yes: Uint128(0), no: Uint128(500) };
        votes_w(&mut deps.storage, bid_id).save(to_binary(&env.message.sender).unwrap().as_slice(), &old_vote).unwrap();

        let mut votes_total = TotalVotes {
            yes: Uint128(2000),
            no: Uint128(1500),
        };
        let old_votes_total = votes_total.clone();


        // load staked ftokens of sender
        let sender = to_binary(&env.message.sender).unwrap();
        let sender_u8 = sender.as_slice();
        // let ftkn_stake = ftkn_stake_r(&deps.storage).load(sender_u8).unwrap();
        let ftkn_stake = StakedTokens { amount: Uint128(1_000), unlock_height: 1_000_000 };

        // check if sender has voted before
        let vote_op = votes_r(&deps.storage, bid_id).may_load(to_binary(&env.message.sender).unwrap().as_slice()).unwrap();
        let old_vote_reg = match vote_op {
            Some(i) => i,
            None => VoteRegister::default(),
        };

        // save new vote
        let mut vote_reg = VoteRegister::default();
        match vote {
            Vote::Yes => vote_reg.yes = ftkn_stake.amount,
            Vote::No => vote_reg.no = ftkn_stake.amount,
        };
        votes_w(&mut deps.storage, bid_id).save(sender_u8, &vote_reg).unwrap();

        // save net effect on (cumulative) vote total tally
        votes_total.yes = Uint128(votes_total.yes.u128().checked_add(vote_reg.yes.u128()).unwrap())
                            .sub(old_vote_reg.yes).unwrap();
        votes_total.no = Uint128(votes_total.no.u128().checked_add(vote_reg.no.u128()).unwrap())
                            .sub(old_vote_reg.no).unwrap();
        votes_total_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &votes_total).unwrap();

    
        println!(
            "old_votes_total: {:?} \nold_vote_reg: {:?} \nvote_reg: {:?} \nvotes_total: {:?}",
            &old_votes_total, &old_vote_reg, &vote_reg, &votes_total
        );
        // dbg!(&vote_reg);
        // dbg!(&votes_total);

    }
}