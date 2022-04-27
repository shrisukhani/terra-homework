#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    QueryRequest, Response, StdError, StdResult, Uint128, WasmMsg, WasmQuery,
};

use cw2::set_contract_version;
use cw20::{BalanceResponse as cw20_BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use oracle::msg::PriceResponse;

use crate::error::ContractError;
use crate::msg::{BalanceResponse, ExecuteMsg, InstantiateMsg, QueryMsg, TokenAddrResponse};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        owner: info.sender.clone(),
        oracle_address: msg.oracle_address.clone(),
        token_address: msg.token_address.clone(),
    };
    STATE.save(deps.storage, &state)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", &info.sender)
        .add_attribute("token_address", &msg.token_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy {} => try_buy(deps, info, env),
        ExecuteMsg::Withdraw { amount } => try_withdraw(deps, info, env, amount),
    }
}

pub fn try_buy(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    if info.funds.len() != 1 {
        return Err(StdError::GenericErr {
            msg: "Didn't send any funds".to_string(),
        }
        .into());
    }

    if info.funds[0].denom != "uluna".to_string() || info.funds[0].amount.is_zero() {
        return Err(StdError::GenericErr {
            msg: "Didn't send uluna".to_string(),
        }
        .into());
    }

    let uluna_sent = info.funds[0].amount.u128();
    let price_in_luna = query_oracle(deps.as_ref())? as u128;
    let current_aurm_balance = query_balance_aurm(deps.as_ref(), env)?.u128();

    let num_potential_swapped_aurm = uluna_sent / price_in_luna;
    if num_potential_swapped_aurm > current_aurm_balance {
        return Err(StdError::GenericErr {
            msg: "Don't have enough AURM to swap".to_string(),
        }
        .into());
    }

    let token_addr = STATE.load(deps.storage)?.token_address;
    let msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: Uint128::from(num_potential_swapped_aurm),
    };

    Ok(Response::new()
        .add_attribute("method", "try_buy")
        .add_attribute("swapped_uluna", uluna_sent.to_string())
        .add_attribute("swapped_aurm", num_potential_swapped_aurm.to_string())
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_addr.to_string(),
            msg: to_binary(&msg)?,
            funds: vec![],
        })))
}

pub fn query_oracle(deps: Deps) -> Result<u64, ContractError> {
    let State { oracle_address, .. } = STATE.load(deps.storage)?;
    let resp: PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: oracle_address.to_string(),
        msg: to_binary(&oracle::msg::QueryMsg::QueryPrice {})?,
    }))?;
    Ok(resp.price)
}

pub fn query_balance_aurm(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let State { token_address, .. } = STATE.load(deps.storage)?;

    let resp: cw20_BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: env.contract.address.to_string(),
        })?,
    }))?;

    Ok(resp.balance)
}

pub fn try_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    amount: i32,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    if info.sender != state.owner {
        return Err(ContractError::Unauthorized {});
    }

    let self_balance = deps
        .querier
        .query_balance(env.contract.address, String::from("uluna"))?;

    if self_balance.amount.u128() < amount as u128 {
        return Err(StdError::GenericErr {
            msg: "Insufficient funds".to_string(),
        }
        .into());
    }

    let msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount as u128, String::from("uluna"))],
    };

    Ok(Response::new()
        .add_attribute("method", "try_withdraw")
        .add_attribute("amount_transferred", amount.to_string())
        .add_attribute("denom_transferred", self_balance.denom.clone())
        .add_message(CosmosMsg::Bank(msg)))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    // TODO
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance => to_binary(&query_balance(deps)?),
        QueryMsg::GetTokenAddr => to_binary(&query_token_addr(deps)?),
    }
}

pub fn query_token_addr(deps: Deps) -> StdResult<TokenAddrResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(TokenAddrResponse {
        token_address: state.token_address,
    })
}

pub fn query_balance(_deps: Deps) -> StdResult<BalanceResponse> {
    // TODO
    Ok(BalanceResponse { balance: 0 })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{coin, testing::mock_info};

    #[test]
    fn proper_initialization() {
        let info = mock_info("creator", &[]);
        let swapper = mock_info("swapper", &vec![coin(10000000000, "uluna")]);
    }
}
