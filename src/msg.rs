use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub fixed_fee: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    SendDuo { receiver1: Addr, receiver2: Addr },
    Withdraw { amount: Coin },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(GetOwnerResponse)]
    GetOwner {},
    #[returns(GetBalanceResponse)]
    GetBalance { address: String },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetOwnerResponse {
    pub owner: Addr,
}
#[cw_serde]
pub struct GetBalanceResponse {
    pub balance: Coin,
}
