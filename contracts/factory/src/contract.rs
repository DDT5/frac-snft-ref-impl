use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, from_binary,
    CosmosMsg, WasmMsg, log, Uint128,
    StdResult, Storage, HumanAddr, // StdError, 
};
use secret_toolkit::utils::{HandleCallback, InitCallback}; //pad_handle_result, pad_query_result,   
// use secret_toolkit::serialization::{Bincode2, Serde};

use crate::msg::{
    InitMsg, HandleMsg, InitFtoken, InterContrMsg, QueryMsg, CountResponse,
    InitialBalance,
};

use fsnft_utils::{FtokenConfig, ContractInfo,};

use crate::state::{RegContr, config, config_read};

pub const BLOCK_SIZE: usize = 256;


/////////////////////////////////////////////////////////////////////////////////
// Init function
/////////////////////////////////////////////////////////////////////////////////

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let mut conf = config(&mut deps.storage);
    conf.save(&RegContr {
        known_snip_721: vec![],
    })?;

    debug_print!("Contract was initialized by {}", env.message.sender);

    Ok(InitResponse::default())
}


/////////////////////////////////////////////////////////////////////////////////
// Handle functions
/////////////////////////////////////////////////////////////////////////////////

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Register { 
            reg_addr, 
            reg_hash 
        } => try_register(
            deps,
            env,
            reg_addr, 
            reg_hash,
        ),
        HandleMsg::BatchReceiveNft { 
            sender, 
            from, 
            token_ids, 
            msg 
        } => try_batch_receive_nft(
            deps,
            env,
            sender, 
            from, 
            token_ids, 
            msg,
        ),
        HandleMsg::TransferNft {
            nft_contr_addr,
            nft_contr_hash,
            recipient,
            token_id
        } => try_transfer_nft(
            deps, 
            env, 
            nft_contr_addr,
            nft_contr_hash,
            recipient,
            token_id,
        ),
        HandleMsg::SendNft {
            nft_contr_addr,
            nft_contr_hash,
            contract,
            token_id,
            msg,
        } => try_send_nft(
            deps,
            env,
            nft_contr_addr,
            nft_contr_hash,
            contract,
            token_id,
            msg,
        ),
        HandleMsg::InstantiateFtoken {
            name,
            symbol,
            decimals,
            callback_code_hash
        } => try_instantiate_ftoken_contr(
            deps,
            env,
            name,
            symbol,
            decimals,
            callback_code_hash,
        ),
        HandleMsg::RegisterFtoken {
            ftoken_config,
            contract_info,
        } => try_register_ftoken(
            deps,
            env,
            ftoken_config,
            contract_info,
        )
    }
}

pub fn try_register<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    reg_addr: HumanAddr,
    reg_hash: String,
) -> StdResult<HandleResponse> {
    let mut conf = config(&mut deps.storage);
    let mut reg_rec = conf.load()?;
    if !reg_rec.known_snip_721.contains(&reg_addr) {
        reg_rec.known_snip_721.push(reg_addr.clone());
    }
    conf.save(&reg_rec)?;

    let msg = to_binary(&InterContrMsg::register_receive(env.contract_code_hash))?;
    let message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: reg_addr,
        callback_code_hash: reg_hash,
        msg,
        send: vec![],
    });

    Ok(HandleResponse {
        messages: vec![message],
        log: vec![],
        data: None,
    })
    
    // Ok(HandleResponse::default())
}

pub fn try_batch_receive_nft<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    sender: HumanAddr,
    from: HumanAddr,
    token_ids: Vec<String>,
    msg: Option<Binary>,
) -> StdResult<HandleResponse> {
    
    debug_print!("Received. Sender: {}, from: {} , token_ids: {:?} , msg: {:?}", &sender, &from, &token_ids, &msg);
    // let msg_deserialized = String::from_utf8(msg.unwrap().as_slice().to_vec()).expect("cannot");
    let msg_deserialized: String = from_binary(&msg.unwrap())?;
    
    let log_msg = vec![
        log("sender", sender),
        log("from", from),
        log("token_ids", format!("{:?}", token_ids)),
        log("msg", format!("{:?}", msg_deserialized))  
    ];
    
    Ok(HandleResponse {
        messages: vec![],
        log: log_msg,
        data: None,
    })
}

pub fn try_transfer_nft<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    nft_contr_addr: HumanAddr,
    nft_contr_hash: String,
    recipient: HumanAddr,
    token_id: String,
) -> StdResult<HandleResponse> {
        // Send message (callback) to NFT contract
        let contract_msg = InterContrMsg::TransferNft {
            recipient: recipient, 
            token_id: token_id,
        };
    
        let cosmos_msg = contract_msg.to_cosmos_msg(
            nft_contr_hash.to_string(), 
            HumanAddr(nft_contr_addr.to_string()), 
            None
        )?;
    
        // create messages
        let messages = vec![cosmos_msg];
    
        Ok(HandleResponse {
            messages: messages,
            log: vec![],
            data: None
        })

    // debug_print!("sender = {}", env.message.sender);
    // debug_print("count incremented successfully");
    // Ok(HandleResponse::default())
}

pub fn try_send_nft<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    nft_contr_addr: HumanAddr,
    nft_contr_hash: String,
    contract: HumanAddr,
    token_id: String,
    _msg: Option<Binary>,
) -> StdResult<HandleResponse> {
    // Send message (callback) to NFT contract
    // let contract_msg = InterContrMsg::SendNft {
    //     contract: contract, 
    //     token_id: token_id,
    //     msg: msg,
    // };

    let msg = Some(to_binary("data from contract here")?);
    let contract_msg = InterContrMsg::SendNft {
            contract: contract, 
            token_id: token_id,
            msg: msg,
        };

    let cosmos_msg = contract_msg.to_cosmos_msg(
        nft_contr_hash.to_string(), 
        HumanAddr(nft_contr_addr.to_string()), 
        None
    )?;

    // create messages
    let messages = vec![cosmos_msg];

    Ok(HandleResponse {
        messages: messages,
        log: vec![],
        data: None
    })
}

pub fn try_instantiate_ftoken_contr<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    symbol: String,
    decimals: u8,
    callback_code_hash: String,
) -> StdResult<HandleResponse> {

    let init_balance = vec![InitialBalance {
        address: env.message.sender,  
        amount: Uint128(1_000_000),
    }];
    let prng_seed = to_binary("prngseed")?;
    
    let contract_msg = InitFtoken {
        name,
        admin: None,
        symbol,
        decimals,
        initial_balances: Some(init_balance),
        prng_seed,
        config: None,
    };
    
    let cosmos_msg = contract_msg.to_cosmos_msg(
        "ftoken_contract".to_string(), 
        3u64, 
        callback_code_hash,
        None,
    )?;
    
    // create messages
    let messages = vec![cosmos_msg];
    
    Ok(HandleResponse {
        messages: messages,
        log: vec![],
        data: None
    })
}

pub fn try_register_ftoken<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _ftoken_config: FtokenConfig,
    _contract_info: ContractInfo,
) -> StdResult<HandleResponse> {
    // authenticate this is message is coming from the expected ftoken contract
    
    // save
    Ok(HandleResponse::default())
}

/////////////////////////////////////////////////////////////////////////////////
// Query functions 
/////////////////////////////////////////////////////////////////////////////////

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.known_snip_721 })
}

/////////////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    // use super::*;
    // use cosmwasm_std::testing::{mock_dependencies, mock_env};
    // use cosmwasm_std::{coins, from_binary, StdError};
}
