mod tests {
    // use super::*;
    use std::any::Any;
    use crate::contract::{init,}; //, handle, query
    use crate::msg::{InitMsg};
    use crate::state::{
        config_r, ftkn_idx_r, ftkn_id_hash_r,
        UploadedFtkn,
    };
    
    use cosmwasm_std::{
        InitResponse, HandleResponse, StdResult, StdError,
        Extern,
    };

    use cosmwasm_std::testing::{mock_dependencies, mock_env, MockStorage, MockApi, MockQuerier};

    fn init_helper_default() -> (
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

    #[test]
    fn test_init_sanity() {
        let (init_result, deps) = init_helper_default();
        assert_eq!(init_result.unwrap(), InitResponse::default());
        let reg_contr = config_r(&deps.storage).load().unwrap();
        assert_eq!(reg_contr.known_snip_721, vec![]);
        assert_eq!(ftkn_idx_r(&deps.storage).load().unwrap(), 0u32);
        assert_eq!(ftkn_id_hash_r(&deps.storage).load().unwrap(), UploadedFtkn::default());
    }   

    #[test]
    fn test_fractionalization_works() {
        // todo
    }
    
}