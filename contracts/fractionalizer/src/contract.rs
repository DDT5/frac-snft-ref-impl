use std::ops::Add;

use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, from_binary,
    CosmosMsg, WasmMsg, log, Uint128,
    StdResult, StdError, Storage, HumanAddr,  
};
use secret_toolkit::utils::{HandleCallback, InitCallback}; //pad_handle_result, pad_query_result,   
// use secret_toolkit::serialization::{Bincode2, Serde};

use crate::msg::{
    InitMsg, HandleMsg, InitFtoken, InterContrMsg, QueryMsg, CountResponse,
    InitialBalance,
};
use crate::state::{
    ftkn_idx_w, ftkn_idx_r, ftoken_contr_w, pending_reg_w, pending_reg_r, ftkn_id_hash_w, ftkn_id_hash_r,
};

use fsnft_utils::{FtokenConf, FtokenContrInit, FtokenInfo, UndrNftInfo};

use crate::state::{Config, config_w, config_r};

pub const BLOCK_SIZE: usize = 256;


/////////////////////////////////////////////////////////////////////////////////
// Init function
/////////////////////////////////////////////////////////////////////////////////

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    config_w(&mut deps.storage).save(&Config {known_snip_721: vec![]})?;
    ftkn_idx_w(&mut deps.storage).save(&0u32)?;
    ftkn_id_hash_w(&mut deps.storage).save(&msg.uploaded_ftoken)?;

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
        // HandleMsg::RegisterNftContr { 
        //     reg_hash ,
        //     reg_addr, 
        // } => try_register_nft_contr(
        //     deps,
        //     env,
        //     reg_hash,
        //     reg_addr, 
        // ),
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
        // HandleMsg::SendNft {
        //     nft_contr_addr,
        //     nft_contr_hash,
        //     contract,
        //     token_id,
        //     msg,
        // } => try_send_nft(
        //     deps,
        //     env,
        //     nft_contr_addr,
        //     nft_contr_hash,
        //     contract,
        //     token_id,
        //     msg,
        // ),
        // HandleMsg::InstantiateFtoken {
        //     name,
        //     symbol,
        //     decimals,
        //     callback_code_hash
        // } => try_instantiate_ftoken_contr(
        //     deps,
        //     env,
        //     name,
        //     symbol,
        //     decimals,
        //     callback_code_hash,
        // ),
        HandleMsg::ReceiveFtokenCallback {
            ftoken_contr,
        } => try_receive_ftoken_callback(
            deps,
            env,
            ftoken_contr,
        ),
        HandleMsg::Fractionalize {
            nft_info,
            ftkn_conf,
        } => try_fractionalize(
            deps,
            env,
            nft_info, 
            ftkn_conf,
        ),
    }
}

/// internal function to register with the SNIP721 contract
fn register_nft_contr_msg<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    reg_hash: &String,
    reg_addr: &HumanAddr,
) -> StdResult<CosmosMsg> {
    let mut conf = config_w(&mut deps.storage);
    let mut reg_rec = conf.load()?;
    if !reg_rec.known_snip_721.contains(&reg_addr) {
        reg_rec.known_snip_721.push(reg_addr.clone());
    }
    conf.save(&reg_rec)?;

    let msg = to_binary(&InterContrMsg::register_receive(&env.contract_code_hash))?;
    let message = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: reg_addr.clone(),
        callback_code_hash: reg_hash.clone(),
        msg,
        send: vec![],
    });
    Ok(message)

    // Ok(HandleResponse {
    //     messages: vec![message],
    //     log: vec![],
    //     data: None,
    // })   
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
            recipient, 
            token_id,
        };
    
        let cosmos_msg = contract_msg.to_cosmos_msg(
            nft_contr_hash, 
            HumanAddr(nft_contr_addr.to_string()), 
            None
        )?;
    
        // create messages
        let messages = vec![cosmos_msg];
    
        Ok(HandleResponse {
            messages,
            log: vec![],
            data: None
        })

    // debug_print!("sender = {}", env.message.sender);
    // debug_print("count incremented successfully");
    // Ok(HandleResponse::default())
}

/// Creates a `SendNft` cosmos msg to be sent to NFT contract 
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `contract` - HumanAddr (String) of receiver of nft, ie: ftoken contract address
fn send_nft_msg<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    nft_contr_addr: HumanAddr,
    nft_contr_hash: String,
    contract: HumanAddr,
    token_id: String,
    msg: Option<Binary>,
) -> StdResult<CosmosMsg> {

    let contract_msg = InterContrMsg::SendNft {
            // address of recipient of nft
            contract, 
            token_id,
            msg,
        };

    let cosmos_msg = contract_msg.to_cosmos_msg(
        nft_contr_hash, 
        HumanAddr(nft_contr_addr.to_string()), 
        None
    )?;

    Ok(cosmos_msg)

    // // create messages
    // let messages = vec![cosmos_msg];

    // Ok(HandleResponse {
    //     messages,
    //     log: vec![],
    //     data: None
    // })
}

/// Generates cosmos message to instantitate a new ftoken contract
/// * `deps` - mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `callback_code_hash` - String holding the code hash of the ftoken contract to be instantiated
fn instantiate_ftoken_contr_msg<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    symbol: String,
    decimals: u8,
    callback_code_hash: String,
    nft_info: UndrNftInfo,
) -> StdResult<CosmosMsg> {
    // log depositor info so can verify when receive callback from ftoken contract
    pending_reg_w(&mut deps.storage).save(&env.message.sender)?;
    
    // create cosmos message
    let init_balance = vec![InitialBalance {
        address: env.message.sender.clone(),  
        amount: Uint128(1_000_000),
    }];
    // optionally use Secret Orcale RNG (scrt-rng) for higher security
    let prng_seed = to_binary("prngseed")?;
    let idx = ftkn_idx_r(&deps.storage).load()?;
    // add one to idx
    ftkn_idx_w(&mut deps.storage).save(&idx.add(1))?;

    let contract_msg = InitFtoken {
        init_info: FtokenContrInit {
            idx,
            depositor: env.message.sender,
            fract_hash: env.contract_code_hash,
            nft_info,
        },
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
        ftkn_id_hash_r(&deps.storage).load()?.code_id,
        callback_code_hash,
        None,
    )?;
    
    Ok(cosmos_msg)

    // Ok(HandleResponse {
    //     messages,
    //     log: vec![],
    //     data: None
    // })
}

/// Receives InitResponse from ftoken contract
/// Registers the ftoken contract, then sends NFT from depositor to ftoken contract inventory
pub fn try_receive_ftoken_callback<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    ftoken_contr: FtokenInfo,
) -> StdResult<HandleResponse> {
    // authenticate this is message is coming from the expected ftoken contract
    let exp_depositor = pending_reg_r(&mut deps.storage).load()?;
    if exp_depositor != ftoken_contr.depositor {
        return Err(StdError::generic_err(
            "Depositor does not match expected depositor",
        ));
    } 
    // remove entry of pending reg
    pending_reg_w(&mut deps.storage).remove(); 

    // save ftoken contract info
    let idx = ftoken_contr.idx;
    ftoken_contr_w(&mut deps.storage).save(&idx.to_le_bytes(), &ftoken_contr)?;

    // `send` NFT from user to ftoken contract
    // does not check if user has given permission to transfer token, because ftoken contract will 
    // perform this check and throw an error if it does not receive the nft  (todo)
    
    let msg = Some(to_binary(&ftoken_contr.nft_info)?);

    let send_nft_msg = send_nft_msg(
        deps, 
        env, 
        ftoken_contr.nft_info.nft_contr.address, 
        ftoken_contr.nft_info.nft_contr.code_hash, 
        ftoken_contr.ftoken_contr.address,  
        ftoken_contr.nft_info.token_id, 
        msg,
    )?;

    // // responds to user with i) ftoken idx, ii) the address of the ftoken contract... user should be able to query this contract any time to get required info

    // // unit tests
    // // - make sure info saved correctly in storage 
    // // -

    // generate cosmosmsg vector
    let messages = vec![send_nft_msg];

    Ok(HandleResponse {
        messages,
        log: vec![
            // log("ftoken_contr", format!("{:?}", ftoken_contr)),
        ],
        data: None,
    })
}

pub fn try_fractionalize<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    nft_info: UndrNftInfo,
    ftkn_conf: FtokenConf,
) -> StdResult<HandleResponse> {
    // register receive with the SNIP721 contract
    // may save gas by first checking if already register received -- not implemented here
    let nft_reg_msg = register_nft_contr_msg(
        deps, 
        &env, 
        &nft_info.nft_contr.code_hash, 
        &nft_info.nft_contr.address
    )?;

    // instantiate new ftoken contract (which should trigger ftoken to callback with RegisterFtoken)
    let ftkn_code_hash = ftkn_id_hash_r(&deps.storage).load()?.code_hash;
    let ftoken_init_msg = instantiate_ftoken_contr_msg(
        deps, 
        env, 
        ftkn_conf.name, 
        ftkn_conf.symbol, 
        ftkn_conf.decimals, 
        ftkn_code_hash,
        nft_info,
    )?;

    // generate message vector
    let mut messages = vec![];
    messages.push(nft_reg_msg);
    messages.push(ftoken_init_msg);

    Ok(HandleResponse {
        messages,
        log: vec![],
        data: None
    })
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
    let state = config_r(&deps.storage).load()?;
    Ok(CountResponse { count: state.known_snip_721 })
}

