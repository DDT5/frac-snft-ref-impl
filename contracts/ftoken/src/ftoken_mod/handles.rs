use std::ops::{Mul, Add};
use uint::{construct_uint};

use cosmwasm_std::{
    log, Api, Binary, Env, Extern, Uint128,
    HandleResponse, HumanAddr, Querier, StdError,
    StdResult, Storage,
    from_binary, to_binary,
    CosmosMsg, WasmMsg, 
};

use crate::{
    contract::{try_transfer_impl},
    msg::{InitMsg},
    state::{Balances, Config}, 
    receiver::Snip20ReceiveMsg,
    ftoken_mod::{
        state::{
        nft_vk_w, nft_vk_r,
        ftoken_contr_s_w, ftoken_contr_s_r, allowed_bid_tokens_r, bids_w, bids_r,
        won_bid_id_w, won_bid_id_r,
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

use super::state::{bid_id_r, bid_id_w, allowed_bid_tokens_w};


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
    allowed_bid_tokens_w(&mut deps.storage).save(&msg.init_info.bid_token)?;

    // InitResponse to fractionalizer contract to register this ftoken contract
    let reg_msg = InitRes::register_receive(msg.clone(), env.clone());
    let cosmos_msg_reg = reg_msg.to_cosmos_msg(
        msg.init_info.fract_hash,
        env.message.sender.clone(),
        None,
    )?; 

    // save ftoken_info
    let val =  match reg_msg {
        InitRes::ReceiveFtokenCallback { ftoken_contr } => ftoken_contr,
        _ => return Err(StdError::generic_err("ftoken contract failed to create register receive message")),
    };

    ftoken_contr_s_w(&mut deps.storage).save(&val)?;

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
    let ftkn_info = ftoken_contr_s_r(&deps.storage).load()?;

    // load SNIP20 token ContractInfo
    let token = allowed_bid_tokens_r(&deps.storage).load()?;

    // create `SendFrom` msg to send to SNIP20 ("sSCRT") contract
    let message = snip20_sendfrom_msg(
        env.message.sender,
        env.contract.address,
        Some(env.contract_code_hash),
        amount,
        Some(to_binary(&ftkn_info)?),
        token.address,
        token.code_hash
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
    // load allowed bid token
    let token = allowed_bid_tokens_r(&deps.storage).load()?;

    // security check: triggered by this contract (ie: the Receive{sender} account is this contract)
    if snip20receivemsg.sender != env.contract.address {
        return Err(StdError::generic_err(
            "`HandleMsg::Receive` is not a public interface. The `sender` of the receive message must be this contract itself"
        ));
    }
    // security check: comes from allowed (eg: sSCRT) token contract
    if env.message.sender != token.address {
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
    };
    // Note that bid_id: u32 implements copy, hence no borrowing issues here
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    // add 1 to bid_id count
    bid_id_w(&mut deps.storage).save(&bid_id.add(1u32))?;
    
    Ok(HandleResponse::default())
}

/// TEMPORARY function to be eventually deleted
/// Allows user to directly change bid status with JSON messages
/// * `bid_idx` - the bid index
/// * `status` - the status to change to. A `u8` number that corresponds to the desired status 
pub fn try_change_bid_status<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    bid_id: u32,
    status_idx: u8,
    winning_bid: Option<u32>,
) -> StdResult<HandleResponse> {
    let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
    let status = match status_idx {
        0 => BidStatus::WonRetrieved,
        1 => BidStatus::WonInVault,
        2 => BidStatus::Active,
        3 => BidStatus::LostInTreasury,
        4 => BidStatus::LostRetrieved,  
        _ => BidStatus::Active, // temp... ie: if invalid index, just keep it at BidStatus::Active, which is the default
    };
    bid_info.status = status;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    // save winning bid
    if let Some(won_bid_id) = winning_bid {
        won_bid_id_w(&mut deps.storage).save(&won_bid_id)?;
    }

    Ok(HandleResponse::default())
}

/// Changes a bid status
/// * `bid_idx` - the bid index
/// * `status` - the status to change to
pub fn _change_bid_status<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    bid_id: u32,
    status: BidStatus,
) -> StdResult<HandleResponse> {
    let mut bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
    bid_info.status = status;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;
    Ok(HandleResponse::default())
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
            "You are not the bidder"
        ));
    }; 

    // check that bid had won and nft is still in vault
    if bid_info.status != BidStatus::WonInVault {
        return Err(StdError::generic_err(
            "Cannot retrieve underlying NFT"
        ));
    }

    // change state to indicate that underlying NFT has been retrieved
    bid_info.status = BidStatus::WonRetrieved;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;

    // create CosmosMsg to NFT contract to transfer NFT to bid winner
    let ftoken_info = ftoken_contr_s_r(&deps.storage).load()?;
    
    let sender = env.message.sender.clone();
    let send_nft_msg = send_nft_msg(
        deps, 
        env, 
        ftoken_info.nft_info.nft_contr.address, 
        ftoken_info.nft_info.nft_contr.code_hash, 
        sender, 
        ftoken_info.nft_info.token_id, 
        None,
    )?;

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
            "You are not the bidder"
        ));
    }; 

    // check that bid had lost and bid amount is still in treasury
    if bid_info.status != BidStatus::LostInTreasury {
        return Err(StdError::generic_err(
            "Cannot retrieve bid tokens"
        ));
    }

    // change state to indicate that bid tokens have been retrieved
    bid_info.status = BidStatus::LostRetrieved;
    bids_w(&mut deps.storage).save(&bid_id.to_le_bytes(), &bid_info)?;
    
    // create `Send` msg to send to SNIP20 ("sSCRT") contract, to transfer bid back to bidder who lost
    // load SNIP20 token ContractInfo
    let token = allowed_bid_tokens_r(&deps.storage).load()?;
    let amount = bid_info.amount;
    
    let message = snip20_send_msg(
        env.message.sender, 
        None, 
        amount, 
        None, 
        token.address, 
        token.code_hash,
    )?;

    let messages = vec![message];

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
    bid_id: u32,
) -> StdResult<HandleResponse> {
    let bid_info = bids_r(&deps.storage).load(&bid_id.to_le_bytes())?;
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
    let won_bid_id = won_bid_id_r(&deps.storage).load()?;
    let bid_info = bids_r(&deps.storage).load(&won_bid_id.to_le_bytes())?;
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
    let token = allowed_bid_tokens_r(&deps.storage).load()?;
    let message = snip20_send_msg(
        env.message.sender, 
        None, 
        pro_rata_proceeds, 
        None, 
        token.address, 
        token.code_hash
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
    let ftoken_info = ftoken_contr_s_r(&deps.storage).load()?;
    if undr_nft != ftoken_info.nft_info {
        return Err(StdError::generic_err("underling NFT info does not match"))
    }

    // verify sender is the expected SNIP721 contract
    let nft_contr = ftoken_info.nft_info.nft_contr;
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
    //         allowed_tokens: todo!(),
    //         permit_name: todo!(),
    //         chain_id: todo!(),
    //         permissions: todo!(),
    //     },
    //     signature: PermitSignature {
    //         pub_key: todo!(),
    //         signature: todo!(),
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
/// * `owner` - token transfers from this address
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

/////////////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use std::ops::{Mul};

    // use super::*;
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
}