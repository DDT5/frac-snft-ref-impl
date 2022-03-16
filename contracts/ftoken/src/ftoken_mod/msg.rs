use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    HumanAddr, Uint128, Env,
};

use fsnft_utils::{FtokenInfo, ContractInfo, UndrNftInfo};
use secret_toolkit::{
    utils::{HandleCallback},
}; 
use crate::{
    contract::{RESPONSE_BLOCK_SIZE}, 
    msg::InitMsg,
};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct InitialBalance {
    pub address: HumanAddr,
    pub amount: Uint128,
}

/// Init Callback response to send upon instantiation of ftoken contract
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum InitRes {
    /// Callback to fractionalizer contract upon instantiation of ftoken contract
    ReceiveFtokenCallback {
        ftoken_contr: FtokenInfo,
    },
    /// set viewing key sent to nft contract
    SetViewingKey {
        /// desired viewing key
        key: String,
        /// optional message length padding
        padding: Option<String>,
    },
}

impl HandleCallback for InitRes {
    const BLOCK_SIZE: usize = RESPONSE_BLOCK_SIZE;
}

/// Implements register_receieve of ftoken contract on fractionalizer
impl InitRes {
    pub fn register_receive(msg: InitMsg, env: Env) -> Self {
        InitRes::ReceiveFtokenCallback {
            ftoken_contr: FtokenInfo {
                ftkn_idx: msg.init_info.ftkn_idx,
                depositor: msg.init_info.depositor,
                ftoken_contr: ContractInfo { 
                    code_hash: env.contract_code_hash, 
                    address: env.contract.address,
                },
                nft_info: UndrNftInfo {
                    token_id: msg.init_info.nft_info.token_id,
                    nft_contr: ContractInfo {
                        code_hash: msg.init_info.nft_info.nft_contr.code_hash,
                        address: msg.init_info.nft_info.nft_contr.address,
                    },
                },
                name: msg.name,
                symbol: msg.symbol,
                decimals: msg.decimals,
                /// set to true at init, as NFT should enter vault at initial deposit, otherwise the tx should fail
                in_vault: true,
                
            }
        }
    }
}