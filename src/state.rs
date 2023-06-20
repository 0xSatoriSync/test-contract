use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub fixed_fees: Coin,
    pub balances: std::collections::HashMap<String, Coin>,
}

pub const STATE: Item<State> = Item::new("state");
