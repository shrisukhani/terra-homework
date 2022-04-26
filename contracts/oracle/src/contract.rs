#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::state::{State, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oracle";
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
        price: msg.price,
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", &state.owner)
        .add_attribute("price", &state.price.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdatePrice { price } => try_update_price(deps, info, price),
    }
}

pub fn try_update_price(
    deps: DepsMut,
    info: MessageInfo,
    new_price: u64,
) -> Result<Response, ContractError> {
    let State {
        owner,
        price: current_price,
    } = STATE.load(deps.storage)?;
    let sender_addr = deps.api.addr_validate(&info.sender.as_str())?;

    if owner != sender_addr {
        return Err(ContractError::Unauthorized {});
    }

    let state = STATE.update::<_, StdError>(deps.storage, |s| {
        Ok(State {
            owner: s.owner,
            price: new_price,
        })
    })?;

    Ok(Response::new()
        .add_attribute("method", "try_update_price")
        .add_attribute("owner", &state.owner)
        .add_attribute("old_price", &current_price.to_string())
        .add_attribute("new_price", &state.price.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryPrice {} => Ok(to_binary(&query_price(deps)?)?),
    }
}

pub fn query_price(deps: Deps) -> StdResult<PriceResponse> {
    let State { price, .. } = STATE.load(deps.storage)?;
    Ok(PriceResponse { price })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Attribute};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { price: 17 };
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res: PriceResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap())
                .unwrap();
        assert_eq!(17, res.price);
    }

    #[test]
    fn error_on_unauthorized_update() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { price: 10 };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try updating price from a non-owner address
        let msg = ExecuteMsg::UpdatePrice { price: 58 };
        let info = mock_info("not_creator", &[]);

        // Check that Unauthorized Error is thrown
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("Expected ContractError::Unauthorized. Got something different."),
        }

        // Check that price is unchanged from instantiation
        let res: PriceResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap())
                .unwrap();
        assert_eq!(10, res.price);
    }

    #[test]
    fn owner_can_update() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg { price: 10 };
        let info = mock_info("creator", &[]);

        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try updating price from owner
        let info = mock_info("creator", &[]);
        let msg = ExecuteMsg::UpdatePrice { price: 58 };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check response attributes
        let Attribute { key, value } = res.attributes.get(0).unwrap();
        assert_eq!("method", key);
        assert_eq!("try_update_price", value);

        let Attribute { key, value } = res.attributes.get(1).unwrap();
        assert_eq!("owner", key);
        assert_eq!("creator", value);

        let Attribute { key, value } = res.attributes.get(2).unwrap();
        assert_eq!("old_price", key);
        assert_eq!("10", value);

        let Attribute { key, value } = res.attributes.get(3).unwrap();
        assert_eq!("new_price", key);
        assert_eq!("58", value);

        // Check that price is updated to new value
        let res: PriceResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::QueryPrice {}).unwrap())
                .unwrap();
        assert_eq!(58, res.price);
    }
}
