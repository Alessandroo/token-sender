use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_json_binary};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:token-sender";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.debug("WASMDEBUG: instantiate");

    let state = State {
        count: msg.limit,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.limit.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::IncrementLimit {} => execute::increment_limit(deps),
        ExecuteMsg::UpdateLimit { limit } => execute::update_limit(deps, limit),
        ExecuteMsg::UpdateLimitWithoutCheck { limit } => execute::update_limit_without_check(deps, limit),
        ExecuteMsg::SendTokens { recipient, amount } => execute::try_send_tokens(deps, recipient, amount),
        ExecuteMsg::TransferTokens { sender, recipient, amount } => {
            execute::execute_transfer_tokens(deps, env, info, sender, recipient, amount)
        }

    }
}

pub mod execute {
    use cosmwasm_std::{BalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, Uint128};

    use super::*;

    pub fn increment_limit(deps: DepsMut) -> Result<Response, ContractError> {
        let mut config = STATE.load(deps.storage)?;
        config.count += Uint128::new(1);
        STATE.save(deps.storage, &config)?;

        Ok(Response::new())
    }

    pub fn update_limit(deps: DepsMut, limit: Uint128) -> Result<Response, ContractError> {
        deps.api.debug("WASMDEBUG: update_limit");

        // Query the contract's balance in a specific denomination
        let denom = "token".to_string();  // Replace with your actual token denom
        let state = STATE.load(deps.storage)?;
        let balance = query_balance(deps.as_ref(), state.owner.to_string(), denom)?;

        deps.api.debug(
            format!(
                "WASMDEBUG: current balance: {:?}",
                balance,
            )
                .as_str(),
        );

        if limit > balance {
            return Err(ContractError::NotEnoughTokens {});
        }

        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count = limit;
            Ok(state)
        })?;
        Ok(Response::new().add_attribute("action", "update_limit"))
    }

    pub fn update_limit_without_check(deps: DepsMut, limit: Uint128) -> Result<Response, ContractError> {
        STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
            state.count = limit;
            Ok(state)
        })?;
        Ok(Response::new().add_attribute("action", "update_limit_without_check"))
    }

    /// Query function to get balance of a specific address and denomination
    fn query_balance(deps: Deps, address: String, denom: String) -> StdResult<Uint128> {
        let balance_query = BankQuery::Balance { denom, address };
        let balance_response: BalanceResponse = deps.querier.query(&balance_query.into())?;
        let balance_coin = balance_response.amount;
        let balance_u128 = balance_coin.amount;
        Ok(balance_u128)
    }

    pub fn try_send_tokens(
        deps: DepsMut,
        recipient: String,
        amount: Uint128, // Uint128 amount
    ) -> Result<Response, ContractError> {
        deps.api.debug("WASMDEBUG: try_send_tokens");

        // Convert recipient to a validated address
        let recipient_addr = deps.api.addr_validate(&recipient)?;

        // Define the denomination of the token, e.g., "token"
        let denom = "token".to_string();

        // Create a Coin from the Uint128 amount
        let coin = Coin {
            denom,
            amount,
        };

        // Create a Vec<Coin> from the Coin
        let amount_vec = vec![coin];

        // Create a BankMsg::Send message
        let bank_msg = BankMsg::Send {
            to_address: recipient_addr.to_string(),
            amount: amount_vec,
        };

        // Create a CosmosMsg::Bank message and include it in the response
        let cosmos_msg: CosmosMsg = bank_msg.into();

        // Return the message in the response
        Ok(Response::new().add_message(cosmos_msg).add_attribute("action", "send_tokens"))
    }

    pub fn execute_transfer_tokens(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        sender: String,
        recipient: String,
        amount: Uint128,
    ) -> Result<Response, ContractError> {
        // Validate sender and recipient addresses
        let sender_addr = deps.api.addr_validate(&sender)?;
        let recipient_addr = deps.api.addr_validate(&recipient)?;

        // Make sure that the one executing this transaction is the actual sender
        if info.sender != sender_addr {
            return Err(ContractError::Unauthorized {});
        }

        // Create a BankMsg::Send message to transfer tokens from sender to recipient
        let transfer_msg = BankMsg::Send {
            to_address: recipient_addr.to_string(),
            amount: vec![Coin {
                denom: "token".to_string(), // Adjust denom as per the chain (e.g., "uosmo", "uatom", etc.)
                amount,
            }],
        };

        let cosmos_msg: CosmosMsg = transfer_msg.into();

        Ok(Response::new()
            .add_message(cosmos_msg)
            .add_attribute("action", "transfer_tokens")
            .add_attribute("sender", sender)
            .add_attribute("recipient", recipient)
            .add_attribute("amount", amount.to_string()))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetLimit {} => to_json_binary(&query::get_limit(deps)?),
        QueryMsg::GetValidator {} => to_json_binary(&query::get_validator(deps)?),
    }
}

pub mod query {
    use crate::msg::{GetLimitResponse, GetValidatorResponse};

    use super::*;

    pub fn get_limit(deps: Deps) -> StdResult<GetLimitResponse> {
        deps.api.debug("WASMDEBUG: get_limit");

        let state = STATE.load(deps.storage)?;
        Ok(GetLimitResponse { limit: state.count })
    }

    pub fn get_validator(deps: Deps) -> StdResult<GetValidatorResponse> {
        deps.api.debug("WASMDEBUG: get_validator");

        let state = STATE.load(deps.storage)?;
        Ok(GetValidatorResponse { validator: state.owner.to_string() })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::SendTokenToContract { amount } => sudo::send_token_to_contract(deps, env, amount),
    }
}

pub mod sudo {
    use cosmwasm_std::{BankMsg, Coin, CosmosMsg, Uint128};

    use super::*;

    pub fn send_token_to_contract(deps: DepsMut, env: Env, amount: Uint128) -> Result<Response, ContractError> {
        deps.api.debug("WASMDEBUG: send_coin_to_contract");

        let mut state = STATE.load(deps.storage)?;
        let contract_address = env.contract.address;

        if amount > state.count {
            return Err(ContractError::NotEnoughTokens {});
        }

        // Create a BankMsg to transfer the coins
        let transfer_amount = Coin {
            denom: "token".to_string(), // Specify the token denomination (e.g., "token")
            amount,         // Transfer the amount provided in the message
        };

        // Construct the bank message for sending tokens
        let bank_msg: CosmosMsg = BankMsg::Send {
            to_address: contract_address.to_string(),  // Recipient address passed via ExecuteMsg
            amount: vec![transfer_amount],      // A vector of the amount to be transferred
        }.into();

        state.count -= amount;
        STATE.save(deps.storage, &state)?;


        Ok(Response::new()
            .add_message(bank_msg)  // Include the bank message
            .add_attribute("action", "transfer")
            .add_attribute("from", state.owner)  // Add attributes for logging
            .add_attribute("to", contract_address)
            .add_attribute("amount", amount))
    }
}
