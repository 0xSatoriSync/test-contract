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
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        owner: msg.owner.clone(),
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
        .add_attribute("owner", msg.owner)
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
        
        // Compute total usei coins sent and check that it's more than the fee
        let total_usei_amount: u128 = info.funds.iter()
            .filter(|c| (*c).denom.eq("usei"))
            .map(|c| c.amount.u128()).sum();
        if state.fixed_fees.amount.u128() >= total_usei_amount {
            return Err(ContractError::InsufficientBalance{});
        }

        // Setup the fees for Transfer to contract owner
        let fee_amount_coin = Coin {
            denom: "usei".into(),
            amount: state.fixed_fees.amount.clone(),
        };

        // Compute amount to send for each account, total_amount - fees and split in half
        let amount_to_send = (total_usei_amount - state.fixed_fees.amount.u128()) / 2;
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
        update_balance(&state.owner, fee_amount_coin);
    
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
        // The Cosmos SDK will process the response message after execution of this function.
        // Thus, the tokens are actually transfered on sucesss.
        let res = Response::new()
            .add_message(BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![amount],
            })
            .add_attribute("method", "withdraw");
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, coins, from_binary, Uint128};

    const TEST_USER1: &str = "TESTUSER1";
    const TEST_USER2: &str = "TESTUSER2";
    const TEST_ADMIN: &str = "TESTADMIN";

    #[test]
    // Note: this tests... 
    // 1) you should be able to instantiate the contract and set the owner
    // 2) you should support a read query to get the owner of the smart contract
    fn proper_initialization() {
        let owner_addr = Addr::unchecked(TEST_ADMIN);

        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { owner: owner_addr.clone(), fixed_fee: Uint128::from(1000u128) };
        let info = mock_info("creator", &coins(10000, "usei"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the owner
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetOwner{}).unwrap();
        let value: GetOwnerResponse = from_binary(&res).unwrap();
        assert_eq!(owner_addr, value.owner);
    }

    #[test]
    // Note: this tests... 
    // 3) you should support an execute message where an account can send coins to the contract and specify two 
    // accounts that can withdraw the coins (for simplicity, split coins evenly across the two destination accounts)
    // 4) you should store the withdrawable coins for every account who has non-zero coins in the contract
    // 5) you should support an execute message where an account can withdraw some or all of its withdrawable coins
    // 6) you should support a read query to get the withdrawable coins of any specified account
    fn send_duo_and_withdraw() {
        let owner_addr = Addr::unchecked(TEST_ADMIN);
        let user1_addr = Addr::unchecked(TEST_USER1);
        let user2_addr = Addr::unchecked(TEST_USER2);

        let mut deps = mock_dependencies();

        // Setup the contract
        let msg = InstantiateMsg { owner: owner_addr.clone(), fixed_fee: Uint128::from(1000u128) };
        let info = mock_info(&owner_addr.to_string(), &coins(101010, "usei"));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len()); // Verify sucessful init

        // Send Tokens to 2 Users: 3000 - 1000(fee) = 1000(user1), 1000(user2)
        let info = mock_info(&owner_addr.to_string(), &coins(3000, "usei"));
        let msg = ExecuteMsg::SendDuo { receiver1: user1_addr.clone(), receiver2: user2_addr.clone() };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len()); // Verify sucessful send

        // Check user1 has half (1000 usei) after contract interaction
        let _info = mock_info(&owner_addr.to_string(), &coins(1, "usei"));
        let msg: QueryMsg = QueryMsg::GetBalance { address: user1_addr.to_string() };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: GetBalanceResponse = from_binary(&res).unwrap();
        assert_eq!(1000u128, value.balance.amount.u128());

        // Check user2 has half (1000 usei) after contract interaction
        let _info = mock_info(&owner_addr.to_string(), &coins(1, "usei"));
        let msg: QueryMsg = QueryMsg::GetBalance { address: user2_addr.to_string() };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: GetBalanceResponse = from_binary(&res).unwrap();
        assert_eq!(1000u128, value.balance.amount.u128());
        
        // Check the contract owner has the fees to collect (1000 usei) after contract interaction
        let _info = mock_info(&owner_addr.to_string(), &coins(1, "usei"));
        let msg: QueryMsg = QueryMsg::GetBalance { address: owner_addr.to_string() };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: GetBalanceResponse = from_binary(&res).unwrap();
        assert_eq!(1000u128, value.balance.amount.u128());

        // Withdraw half of user1's balance
        let info = mock_info(&user1_addr.to_string(), &coins(1, "usei"));
        let msg = ExecuteMsg::Withdraw { amount: Coin{ denom: "usei".to_string(), amount: Uint128::from(500u128) } };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // Check Result
        let _info = mock_info(&user1_addr.to_string(), &coins(1, "usei"));
        let msg: QueryMsg = QueryMsg::GetBalance { address: user1_addr.to_string() };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: GetBalanceResponse = from_binary(&res).unwrap();
        assert_eq!(500u128, value.balance.amount.u128());

        // Withdraw all of user2's balance
        let info = mock_info(&user2_addr.to_string(), &coins(1, "usei"));
        let msg = ExecuteMsg::Withdraw { amount: Coin{ denom: "usei".to_string(), amount: Uint128::from(1000u128) } };
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        // Check Result
        let _info = mock_info(&user2_addr.to_string(), &coins(1, "usei"));
        let msg: QueryMsg = QueryMsg::GetBalance { address: user2_addr.to_string() };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let value: GetBalanceResponse = from_binary(&res).unwrap();
        assert_eq!(0u128, value.balance.amount.u128());

    }
}
