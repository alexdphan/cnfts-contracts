use schemars::JsonSchema;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};

/// Cw721ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
#[cw_serde]
// this Cw721ReceiveMsg contains a CosmosMsg, which is a variant of the ExecuteMsg enum
pub struct Cw721ReceiveMsg {
    pub sender: String,
    pub token_id: String,
    pub msg: Binary,
}

impl Cw721ReceiveMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = ReceiverExecuteMsg::ReceiveNft(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>, C>(self, contract_addr: T) -> StdResult<CosmosMsg<C>>
    where
        C: Clone + std::fmt::Debug + PartialEq + JsonSchema,
    {
        // msg is the serialized binary version of struct/impl Cw721ReceiveMsg (self)
        let msg = self.into_binary()?; 
        let execute = WasmMsg::Execute {
            // attaches the contract address to the message 
            // attaching wasm message to the the cosmos message
            contract_addr: contract_addr.into(), 
            // the serialized binary version of struct/impl Cw721ReceiveMsg (self)
            msg, 
            funds: vec![], // optionally attaches funds
        };
        Ok(execute.into())
    }
}

/// This is just a helper to properly serialize the above message.
/// The actual receiver should include this variant in the larger ExecuteMsg enum
#[cw_serde]
enum ReceiverExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),
}
