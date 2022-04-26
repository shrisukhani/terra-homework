#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult,
};

use cw2::set_contract_version;

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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // TODO
    Err(ContractError::NotImplemented {})
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
    Ok(BalanceResponse { balance: 0 })
}

#[cfg(test)]
mod tests {
    #[test]
    fn proper_initialization() {

        //TODO
    }
}
