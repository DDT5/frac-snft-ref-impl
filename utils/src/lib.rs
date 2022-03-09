use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{HumanAddr};



/////////////////////////////////////////////////////////////////////////////////
// ftoken structs
/////////////////////////////////////////////////////////////////////////////////

/// config of the ftoken contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FtokenConfig {
    pub index: u32,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

/// code hash and address of a contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractInfo {
    /// contract's code hash string
    pub code_hash: String,
    /// contract's address in HumanAddr
    pub address: HumanAddr,
}


/////////////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
