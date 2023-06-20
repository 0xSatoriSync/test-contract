#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, BankMsg};
use cosmwasm_storage::{singleton_read, singleton};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetOwnerResponse, GetBalanceResponse, InstantiateMsg, QueryMsg};
use crate::state::State;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:test-contract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CONFIG_KEY: &[u8] = b"TEST_CONFIG";

// Instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: info.sender.clone(),
        fixed_fees: Coin {
            denom: "usei".into(),
            amount: msg.fixed_fee.into(),
        },
        balances: std::collections::HashMap::new(),
    };
    
    singleton(deps.storage, CONFIG_KEY).save(&state)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("transfer fee (usei)", msg.fixed_fee.to_string()))
}

// Read API
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&query::get_owner(deps)?),
        QueryMsg::GetBalance { address } => to_binary(&query::get_balance(deps, &address)?),
    }
}
pub mod query {
    use super::*;
    pub fn get_owner(deps: Deps) -> StdResult<GetOwnerResponse> {
        // Read the contract state
        let state: State = singleton_read(deps.storage, CONFIG_KEY).load()?;
        // Return owner set at deployment
        Ok(GetOwnerResponse { owner: state.owner })
    }
    pub fn get_balance(deps: Deps, address: &String) -> StdResult<GetBalanceResponse> { 
        // Read the contract state
        let state: State = singleton_read(deps.storage, CONFIG_KEY).load()?;
        // Return specified address balance
        match state.balances.get(address) {
            Some(coin) => Ok(GetBalanceResponse{ balance: coin.clone() }),
            None => Ok(GetBalanceResponse{ balance: Coin::new(0, "usei")}),
        }
    }
}

// Write API
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SendDuo { receiver1, receiver2 } => execute::send_duo(deps, env, info, receiver1, receiver2),
        ExecuteMsg::Withdraw { amount } => execute::withdraw(deps, env, info, amount),
    }
}

pub mod execute {
    use super::*;
    use cosmwasm_std::{Addr, Coin};

    pub fn send_duo(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        rx1: Addr,
        rx2: Addr,
    ) -> Result<Response, ContractError> {
        // Read the contract state
        let mut state: State = singleton_read(deps.storage, CONFIG_KEY).load()?;
        
        // Compute total sent coins and check that it's more than the fee
        let total_amount: u128 = info.funds.iter().map(|c| c.amount.u128()).sum();
        if state.fixed_fees.amount.u128() >= total_amount {
            return Err(ContractError::InsufficientBalance{});
        }

        // Compute amount to send for each account, total_amount - fees and split in half
        let amount_to_send = (total_amount - state.fixed_fees.amount.u128()) / 2;
        let amount_to_send_coin = Coin {
            denom: "usei".into(),
            amount: amount_to_send.into(),
        };
    
        // Update balances for each account
        let mut update_balance = |addr: &Addr, amount: Coin| {
            let current_balance = state.balances.entry(addr.to_string()).or_insert(Coin {
                denom: "usei".into(),
                amount: 0u128.into(),
            });
            current_balance.amount = (current_balance.amount.u128() + amount.amount.u128()).into();
        };
        update_balance(&rx1, amount_to_send_coin.clone());
        update_balance(&rx2, amount_to_send_coin);
    
        // Save updated state
        singleton(deps.storage, CONFIG_KEY).save(&state)?;
        
        // Return sucess
        Ok(Response::new())
    }

   pub fn withdraw(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        amount: Coin,
    ) -> Result<Response, ContractError> {
        // Read the contract state
        let mut state: State = singleton_read(deps.storage, CONFIG_KEY).load()?;

        // Validate the amount
        if amount.amount.is_zero() {
            return Err(ContractError::InvalidArgument{ msg: "Amount cannot be zero".to_string()});
        }
        // Check if the account has enough balance to withdraw
        let balance = state
            .balances
            .get(&info.sender.to_string())
            .ok_or(ContractError::InsufficientBalance{})?;
        if balance.amount.u128() < amount.amount.u128() {
            return Err(ContractError::InsufficientBalance{});
        }
    
        // Deduct the amount from the account's balance
        let current_balance = state.balances.get_mut(&info.sender.to_string()).unwrap();
        current_balance.amount = (current_balance.amount.u128() - amount.amount.u128()).into();
    
        // Save updated state
        singleton(deps.storage, CONFIG_KEY).save(&state)?;
    
        // Create the response message for the withdrawal
        let res = Response::new()
            .add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![amount],
            })
            .add_attribute("method", "withdraw");
        Ok(res)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
//     use cosmwasm_std::{coins, from_binary};

//     #[test]
//     fn proper_initialization() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(1000, "earth"));

//         // we can just call .unwrap() to assert this was a success
//         let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
//         assert_eq!(0, res.messages.len());

//         // it worked, let's query the state
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_binary(&res).unwrap();
//         assert_eq!(17, value.count);
//     }

//     #[test]
//     fn increment() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Increment {};
//         let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // should increase counter by 1
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_binary(&res).unwrap();
//         assert_eq!(18, value.count);
//     }

//     #[test]
//     fn reset() {
//         let mut deps = mock_dependencies();

//         let msg = InstantiateMsg { count: 17 };
//         let info = mock_info("creator", &coins(2, "token"));
//         let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

//         // beneficiary can release it
//         let unauth_info = mock_info("anyone", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
//         match res {
//             Err(ContractError::Unauthorized {}) => {}
//             _ => panic!("Must return unauthorized error"),
//         }

//         // only the original creator can reset the counter
//         let auth_info = mock_info("creator", &coins(2, "token"));
//         let msg = ExecuteMsg::Reset { count: 5 };
//         let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

//         // should now be 5
//         let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
//         let value: GetCountResponse = from_binary(&res).unwrap();
//         assert_eq!(5, value.count);
//     }
// }
