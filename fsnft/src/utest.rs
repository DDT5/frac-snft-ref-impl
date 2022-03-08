#[cfg(test)]
mod tests {
    // use super::*;
    use std::any::Any;
    use crate::contract::{init}; //, handle, query
    use crate::msg::{InitMsg};
    use crate::state::{
        config_read,
    };
    
    use cosmwasm_std::{
        InitResponse, HandleResponse, StdResult, StdError,
        Extern,
    };

    use cosmwasm_std::testing::*;

    fn init_helper_default() -> (
        StdResult<InitResponse>,
        Extern<MockStorage, MockApi, MockQuerier>,
    ) {
        let mut deps = mock_dependencies(20, &[]);
        let env = mock_env("instantiator", &[]);

        let init_msg = InitMsg {
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
        let reg_contr = config_read(&deps.storage).load().unwrap();
        assert_eq!(reg_contr.known_snip_721, vec![])
    }   
}