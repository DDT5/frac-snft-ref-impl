mod tests {
//     // use super::*;
//     use std::any::Any;
//     use crate::contract::{init, handle}; //, query
//     use crate::msg::{InitMsg, HandleMsg, InterContrMsg, InitFtoken, InitialBalance};
//     use crate::state::{
//         config_r, ftkn_idx_r, ftkn_id_hash_r,
//         UploadedFtkn, Config,
//     };
    
//     use cosmwasm_std::{
//         InitResponse, HandleResponse, StdResult, StdError,
//         Extern, HumanAddr, Uint128, to_binary, CosmosMsg, WasmMsg,
//     };

//     use cosmwasm_std::testing::{mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier};
//     use fsnft_utils::{UndrNftInfo, ContractInfo, FtokenConf, FtokenContrInit};
//     use secret_toolkit::utils::InitCallback;


// /////////////////////////////////////////////////////////////////////////////////
// // Helper functions
// /////////////////////////////////////////////////////////////////////////////////

//     fn init_helper_default() -> (
//         StdResult<InitResponse>,
//         Extern<MockStorage, MockApi, MockQuerier>,
//     ) {
//         let mut deps = mock_dependencies(20, &[]);
//         let env = mock_env("instantiator", &[]);

//         let init_msg = InitMsg {
//             uploaded_ftoken: UploadedFtkn::default(),
//         };

//         (init(&mut deps, env, init_msg), deps)
//     }

//     fn _extract_error_msg<T: Any>(error: StdResult<T>) -> String {
//         match error {
//             Ok(_response) => panic!("Expected error, but had Ok response"),
//             Err(err) => match err {
//                 StdError::GenericErr { msg, .. } => msg,
//                 _ => panic!("Unexpected error result {:?}", err),
//             },
//         }
//     }

//     fn _extract_log(resp: StdResult<HandleResponse>) -> String {
//         match resp {
//             Ok(response) => response.log[0].value.clone(),
//             Err(_err) => "These are not the logs you are looking for".to_string(),
//         }
//     }

// /////////////////////////////////////////////////////////////////////////////////
// // Tests
// /////////////////////////////////////////////////////////////////////////////////

//     #[test]
//     fn test_init_sanity() {
//         let (init_result, deps) = init_helper_default();
//         assert_eq!(init_result.unwrap(), InitResponse::default());
//         // check init storage variables
//         assert_eq!(config_r(&deps.storage).load().unwrap(), Config {known_snip_721: vec![]});
//         assert_eq!(ftkn_idx_r(&deps.storage).load().unwrap(), 0u32);
//         assert_eq!(ftkn_id_hash_r(&deps.storage).load().unwrap(), UploadedFtkn::default());
//     }   

//     #[test]
//     fn test_fractionalize_sanity() {
//         // instantiate fract contract 
//         // -----------------------------------------------------------------------------
//         let (_, mut deps) = init_helper_default();
        
//         // call fractionalize handle on fract contract 
//         // -----------------------------------------------------------------------------
//         let supply = Uint128(2_000);
//         let handle_msg = HandleMsg::Fractionalize {
//             nft_info: UndrNftInfo {
//                 token_id: "myNFT".to_string(),
//                 nft_contr: ContractInfo {
//                     code_hash: "nft_hash".to_string(),
//                     address: HumanAddr("nft_addr".to_string()),
//                 },
//             },
//             ftkn_conf: FtokenConf {
//                 name: "myftoken".to_string(),
//                 symbol: "TKN".to_string(),
//                 supply,
//                 decimals: 6u8,
//             },
//         };
//         let env_fract = mock_env("NFTdepositor", &[]);
//         let handle_result = handle(&mut deps, env_fract.clone(), handle_msg);
//         let handle_resp = handle_result.unwrap();
//         // there are two messages in the vector
//         assert_eq!(handle_resp.messages.len(), 2);
//         // messages[0] is..
//         let msg0 = to_binary(&InterContrMsg::register_receive(&env_fract.contract_code_hash)).unwrap();
//         let message0 = CosmosMsg::Wasm(WasmMsg::Execute {
//         contract_addr: HumanAddr("nft_addr".to_string()),
//         callback_code_hash: "nft_hash".to_string(),
//         msg: msg0,
//         send: vec![],
//     });
//         assert_eq!(handle_resp.messages[0], message0);
//         // messages[1] is..
//         let msg1 = InitFtoken {
//             init_info: FtokenContrInit {
//                 idx: 0u32,
//                 depositor: HumanAddr("NFTdepositor".to_string()),
//                 fract_hash: env_fract.contract_code_hash,
//                 nft_info: UndrNftInfo {
//                     token_id: "myNFT".to_string(),
//                     nft_contr: ContractInfo {
//                         code_hash: "nft_hash".to_string(),
//                         address: HumanAddr("nft_addr".to_string()),
//                     },
//                 },
//             },
//             name: "myftoken".to_string(),
//             admin: None,
//             symbol: "TKN".to_string(),
//             decimals: 6u8,
//             initial_balances: Some(vec![InitialBalance {
//                 address: HumanAddr("NFTdepositor".to_string()),  
//                 amount: supply,
//             }]),
//             prng_seed: to_binary("prngseed").unwrap(),
//             config: None,
//         };
//         let message1 = msg1.to_cosmos_msg(
//             "ftoken_contract".to_string(), 
//             ftkn_id_hash_r(&deps.storage).load().unwrap().code_id,
//             "".to_string(),
//             None,
//         ).unwrap();
//         assert_eq!(handle_resp.messages[1], message1);

//         // call init function on ftoken contract 
//         // -----------------------------------------------------------------------------
        
//     }

//     #[test]
//     fn test_receive_ftoken_callback() {
//         // todo
//     }
    
}