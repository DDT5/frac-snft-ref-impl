#![allow(clippy::field_reassign_with_default)] // This is triggered in `#[derive(JsonSchema)]`

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Binary, CosmosMsg, HumanAddr, StdResult, Uint128, WasmMsg};

use crate::{contract::RESPONSE_BLOCK_SIZE};

/// Snip20ReceiveMsg should be de/serialized under `Receive()` variant in a HandleMsg
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Snip20ReceiveMsg {
    pub sender: HumanAddr,
    pub from: HumanAddr,
    pub amount: Uint128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub msg: Option<Binary>,
}

impl Snip20ReceiveMsg {
    pub fn new(
        sender: HumanAddr,
        from: HumanAddr,
        amount: Uint128,
        memo: Option<String>,
        msg: Option<Binary>,
    ) -> Self {
        Self {
            sender,
            from,
            amount,
            memo,
            msg,
        }
    }

    /// serializes the message, and pads it to 256 bytes
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ReceiverHandleMsg::Receive(self);
        let mut data = to_binary(&msg)?;
        space_pad(RESPONSE_BLOCK_SIZE, &mut data.0);
        Ok(data)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg(
        self,
        callback_code_hash: String,
        contract_addr: HumanAddr,
    ) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            msg,
            callback_code_hash,
            contract_addr,
            send: vec![],
        };
        Ok(execute.into())
    }
}

// This is just a helper to properly serialize the above message
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
enum ReceiverHandleMsg {
    Receive(Snip20ReceiveMsg),
}


/// Originally in SNIP20 ref impl msg.rs file, which its receiver.rs refers to
/// Take a Vec<u8> and pad it up to a multiple of `block_size`, using spaces at the end.
pub fn space_pad(block_size: usize, message: &mut Vec<u8>) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}
