// use super::*;
use std::any::Any;
use fractionalizer::{
    contract::{init, handle}, 
    msg::{InitMsg, HandleMsg, InterContrMsg, InitFtoken, InitialBalance}, 
    state::{
        config_r, ftkn_idx_r, ftkn_id_hash_r,
        UploadedFtkn, Config
    },
};
use ftoken::{
    contract::{
        init as ft_init,
    }, 
    msg::{
        InitMsg as FtInitMsg, InitialBalance as FtInitialBalance, InitConfig as FtInitConfig,
    }, 
    state::{ReadonlyBalances, read_ftkn_info},
};

use cosmwasm_std::{
    InitResponse, HandleResponse, StdResult, StdError,
    Extern, HumanAddr, Uint128, to_binary, CosmosMsg, WasmMsg, Binary, Api, 
};

use cosmwasm_std::testing::{mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier};
use fsnft_utils::{UndrNftInfo, ContractInfo, FtokenConf, FtokenContrInit, json_ser_deser, FtokenInfo, more_mock_env};
use secret_toolkit::utils::{InitCallback, HandleCallback};


/////////////////////////////////////////////////////////////////////////////////
// Helper functions
/////////////////////////////////////////////////////////////////////////////////

fn frac_init_helper_default() -> (
    StdResult<InitResponse>,
    Extern<MockStorage, MockApi, MockQuerier>,
) {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("instantiator", &[]);

    let init_msg = InitMsg {
        uploaded_ftoken: UploadedFtkn::default(),
    };

    (init(&mut deps, env, init_msg), deps)
}

fn ftoken_init_helper_with_config(
    init_info: FtokenContrInit,
    name: String,
    admin: Option<HumanAddr>,
    symbol: String,
    decimals: u8,
    initial_balances: Option<Vec<FtInitialBalance>>,
    prng_seed: Binary,
    config: Option<FtInitConfig>,
) -> (
    StdResult<InitResponse>,
    Extern<MockStorage, MockApi, MockQuerier>,
) {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("ftkn_instantiator", &[]);

    let init_msg = FtInitMsg {
        init_info,
        name,
        admin,
        symbol,
        decimals,
        initial_balances,
        prng_seed,
        config,
    };

    (ft_init(&mut deps, env, init_msg), deps)
}

fn _extract_error_msg<T: Any>(error: StdResult<T>) -> String {
    match error {
        Ok(_response) => panic!("Expected error, but had Ok response"),
        Err(err) => match err {
            StdError::GenericErr { msg, .. } => msg,
            _ => panic!("Unexpected error result {:?}", err),
        },
    }
}

fn _extract_log(resp: StdResult<HandleResponse>) -> String {
    match resp {
        Ok(response) => response.log[0].value.clone(),
        Err(_err) => "These are not the logs you are looking for".to_string(),
    }
}

/////////////////////////////////////////////////////////////////////////////////
// Tests
/////////////////////////////////////////////////////////////////////////////////

#[test]
fn test_fract_init_sanity() {
    let (init_result, deps) = frac_init_helper_default();
    assert_eq!(init_result.unwrap(), InitResponse::default());
    // check init storage variables
    assert_eq!(config_r(&deps.storage).load().unwrap(), Config {known_snip_721: vec![]});
    assert_eq!(ftkn_idx_r(&deps.storage).load().unwrap(), 0u32);
    assert_eq!(ftkn_id_hash_r(&deps.storage).load().unwrap(), UploadedFtkn::default());
}   

#[test]
fn test_fractionalize_integration() {
    let nft_depositor_addr = HumanAddr("NFTdepositor".to_string());
    // instantiate fract contract 
    // -----------------------------------------------------------------------------
    let (_, mut deps) = frac_init_helper_default();
    
    // call fractionalize handle on fract contract 
    // -----------------------------------------------------------------------------
    // set variables
    let supply = Uint128(2_000);
    let nft_info = UndrNftInfo {
        token_id: "myNFT".to_string(),
        nft_contr: ContractInfo {
            code_hash: "nft_hash".to_string(),
            address: HumanAddr("nft_addr".to_string()),
        },
    };

    // call fractionalize handle on fract contract
    let handle_msg = HandleMsg::Fractionalize {
        nft_info: nft_info.clone(),
        ftkn_conf: FtokenConf {
            name: "myftoken".to_string(),
            symbol: "TKN".to_string(),
            supply,
            decimals: 6u8,
        },
    };
    let env = mock_env("NFTdepositor", &[]);
    let handle_result = handle(&mut deps, env.clone(), handle_msg);
    let handle_resp = handle_result.unwrap();
    
    // there are two messages in the vector
    assert_eq!(handle_resp.messages.len(), 2);

    // messages[0] is..
    let msg0 = to_binary(&InterContrMsg::register_receive(&env.contract_code_hash)).unwrap();
    let message0 = CosmosMsg::Wasm(WasmMsg::Execute {
    contract_addr: HumanAddr("nft_addr".to_string()),
    callback_code_hash: "nft_hash".to_string(),
    msg: msg0,
    send: vec![],
});
    assert_eq!(handle_resp.messages[0], message0);

    // messages[1] is..
    let exp_msg1 = InitFtoken {
        init_info: FtokenContrInit {
            idx: 0u32,
            depositor: nft_depositor_addr.clone(),
            fract_hash: env.contract_code_hash.clone(),
            nft_info: nft_info.clone(),
        },
        name: "myftoken".to_string(),
        admin: None,
        symbol: "TKN".to_string(),
        decimals: 6u8,
        initial_balances: Some(vec![InitialBalance {
            address: nft_depositor_addr.clone(),  
            amount: supply,
        }]),
        prng_seed: to_binary("prngseed").unwrap(),
        config: None,
    };
    let message1 = exp_msg1.to_cosmos_msg(
        "ftoken_contract".to_string(), 
        ftkn_id_hash_r(&deps.storage).load().unwrap().code_id,
        "".to_string(),
        None,
    ).unwrap();
    assert_eq!(handle_resp.messages[1], message1);

    // SNIP721 successfully register received
    // -----------------------------------------------------------------------------
    // todo!(), low prioritiy



    // contract-to-contract call init function on ftoken contract 
    // -----------------------------------------------------------------------------
    let ft_initial_balance: Option<Vec<FtInitialBalance>> = json_ser_deser(&exp_msg1.initial_balances).unwrap();
    let ft_config: Option<FtInitConfig> = json_ser_deser(&exp_msg1.config).unwrap();
    let ft_env = mock_env("fract_contract", &[]);
    let (ft_init_result, ft_deps) = ftoken_init_helper_with_config(
        exp_msg1.init_info,
        exp_msg1.name,
        exp_msg1.admin,
        exp_msg1.symbol,
        exp_msg1.decimals,
        ft_initial_balance,
        exp_msg1.prng_seed,
        ft_config,
    );
    let ft_init_resp = ft_init_result.unwrap();

    // check initial balance is correct -- that NFT depositor receives the "supply" amount of ftokens
    let depositor_bal = Uint128(ReadonlyBalances::from_storage(&ft_deps.storage).account_amount(
        &ft_deps.api.canonical_address(&nft_depositor_addr).unwrap()
    ));
    assert_eq!(depositor_bal, supply);

    // check that ftokenInfo stored correctly in ftoken contract
    let ft_ftkn_info = read_ftkn_info(&ft_deps.storage).unwrap();
    let exp_ft_ftkn_info = FtokenInfo {
        idx: 0u32,
        depositor: nft_depositor_addr.clone(),
        ftoken_contr: ContractInfo { 
            code_hash: ft_env.contract_code_hash, 
            address: ft_env.contract.address.clone() 
        },
        nft_info: nft_info.clone(),
        name: "myftoken".to_string(),
        symbol: "TKN".to_string(),
        decimals: 6u8,
    };
    assert_eq!(ft_ftkn_info, exp_ft_ftkn_info);

    // check ftoken init function emits two cosmos_msgs
    assert_eq!(ft_init_resp.messages.len(), 2);

    // contract-to-contract call ftoken init response -> fractionalizer handle 
    // -----------------------------------------------------------------------------
    let handle_msg = HandleMsg::ReceiveFtokenCallback {
        ftoken_contr: exp_ft_ftkn_info.clone(),
    };
    let ft_frc_env = more_mock_env(ft_env.contract.address.clone(), None, None);
    let handle_result = handle(&mut deps, ft_frc_env, handle_msg);
    let handle_resp = handle_result.unwrap();
    // check there is one message in the response
    assert_eq!(handle_resp.messages.len(), 1);
    let exp_send_msg = InterContrMsg::SendNft {
        // address of recipient of nft
        contract: ft_env.contract.address, 
        token_id: "myNFT".to_string(),
        msg: Some(to_binary(&nft_info).unwrap()),
    };
    let exp_send_msg_cosm = exp_send_msg.to_cosmos_msg(
        "nft_hash".to_string(),
        HumanAddr("nft_addr".to_string()),
        None
    ).unwrap();
    assert_eq!(handle_resp.messages[0], exp_send_msg_cosm);

    // SNIP721 send handle
    // -----------------------------------------------------------------------------
    // todo!()
}


