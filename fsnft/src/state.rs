use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage}; //CanonicalAddr
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};

pub const CONFIG_KEY: &[u8] = b"config";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RegContr {
    pub known_snip_721: Vec<HumanAddr>,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, RegContr> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, RegContr> {
    singleton_read(storage, CONFIG_KEY)
}
