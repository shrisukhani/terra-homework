#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, wasm_execute, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StakingMsg, StdError, StdResult, SubMsg, Uint128,
};
use cw0::must_pay;
use cw2::set_contract_version;
//use cw20::Cw20ExecuteMsg;

use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};
use terra_cosmwasm::{ExchangeRatesResponse, TerraMsgWrapper, TerraQuerier};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, STATE};

use oracle::msg::{PriceResponse as OraclePriceResponse, QueryMsg::QueryPrice as OracleQueryPrice};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swap2";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// BlockNgine - 0% comission on testnet
const _VALIDATOR: &str = "terravaloper1ze5dxzs4zcm60tg48m9unp8eh7maerma38dl84";

// StakeBin - 1% comission on testnet
// https://finder.terra.money/testnet/validator/terravaloper19ne0aqltndwxl0n32zyuglp2z8mm3nu0gxpfaw
// const VALIDATOR: &str = "terravaloper19ne0aqltndwxl0n32zyuglp2z8mm3nu0gxpfaw";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(
        deps.storage,
        &State {
            oracle_address: msg.oracle_address.clone(),
            token_address: msg.oracle_address.clone(),
            owner: info.sender.clone(),
        },
    )?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    // TODO
    Err(StdError::generic_err("not implemented"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::Buy {} => try_buy(deps, env, info),
        ExecuteMsg::Withdraw { amount } => {
            try_withdraw_step1_collect_rewards(deps, env, info, amount)
        }
        ExecuteMsg::StartUndelegation { amount } => try_start_undelegation(deps, env, info, amount),
        ExecuteMsg::WithdrawStep2ConvertRewardsToLuna { amount } => {
            try_withdraw_step2_convert_all_native_coins_to_luna(deps, env, info, amount)
        }
        ExecuteMsg::WithdrawStep3SendLuna { amount } => {
            try_withdraw_step3_send_luna(deps, env, info, amount)
        }
    }
}

pub fn try_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    // Check payment_amt is in uluna, only 1 coin has been supplied, and it's non-zero
    let payment_amt =
        must_pay(&info, "uluna").map_err(|error| StdError::generic_err(format!("{}", error)))?;

    // Get AURM price in uluna from oracle
    let oracle_price = query_aurm_oracle(deps.as_ref())? as u128;

    // Compute number of AURM tokens user should get at `oracle_price`
    let swap_aurum_qty = payment_amt.u128() / oracle_price;

    // Get swap2's own AURM balance
    let self_aurum_balance = query_own_aurm_balance(deps.as_ref(), env)?;

    // Return a BuyError if contract does not have sufficient AURM to complete txn
    if self_aurum_balance.u128() < swap_aurum_qty {
        return Err(ContractError::BuyError {});
    }

    let state = STATE.load(deps.storage)?;

    // Make message for transferring AURM to user
    let transfer_aurm_to_user_msg = wasm_execute(
        state.token_address,
        &Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount: Uint128::from(swap_aurum_qty),
        },
        vec![],
    )?;

    let msgs = vec![
        // Delegate new luna generated to a validator immediately
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: "validator_friend".to_string(),
            amount: coin(payment_amt.u128(), String::from("uluna")),
        }),
        transfer_aurm_to_user_msg.into(),
    ];

    Ok(Response::<TerraMsgWrapper>::new().add_messages(msgs))
}

pub fn try_withdraw_step1_collect_rewards(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    _amount: u64,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    // Step 1: Collect all rewards we have accrued.

    let _reward_submessages = collect_all_rewards(deps, &env)?;

    // TODO
    Ok(Response::<TerraMsgWrapper>::new())
}

pub fn collect_all_rewards(
    _deps: DepsMut,
    _env: &Env,
) -> Result<Vec<SubMsg<TerraMsgWrapper>>, ContractError> {
    // TODO
    Err(ContractError::NotImplemented {})
}

pub fn try_withdraw_step2_convert_all_native_coins_to_luna(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _amount: u64,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    // TODO
    Err(ContractError::NotImplemented {})
}

pub fn try_withdraw_step3_send_luna(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _amount: u64,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    // TODO
    Err(ContractError::NotImplemented {})
}

pub fn try_start_undelegation(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _amount: Uint128,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    // TODO
    Err(ContractError::NotImplemented {})
}

pub fn query_exchange_rates(
    deps: &DepsMut,
    base_denom: String,
    quote_denoms: Vec<String>,
) -> StdResult<ExchangeRatesResponse> {
    let querier = TerraQuerier::new(&deps.querier);
    let res: ExchangeRatesResponse = querier.query_exchange_rates(base_denom, quote_denoms)?;
    Ok(res)
}

pub fn query_aurm_oracle(deps: Deps) -> Result<u64, ContractError> {
    let state = STATE.load(deps.storage)?;
    let msg = OracleQueryPrice {};
    let resp: OraclePriceResponse = deps.querier.query_wasm_smart(state.oracle_address, &msg)?;
    Ok(resp.price)
}

pub fn query_own_aurm_balance(deps: Deps, env: Env) -> Result<Uint128, ContractError> {
    let state = STATE.load(deps.storage)?;
    let msg = Cw20QueryMsg::Balance {
        address: env.contract.address.to_string(),
    };
    let resp: cw20::BalanceResponse = deps.querier.query_wasm_smart(state.token_address, &msg)?;
    Ok(resp.balance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            token_address: Addr::unchecked("terra1hpajld8zs93md8zrs6sfy42zl0khqpmr07muw0"),
            oracle_address: Addr::unchecked("oracle_addr"),
        };
        let info = mock_info("creator", &coins(10000000000, "uluna"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::QueryTokenAddress {});
        assert_eq!(res, Err(StdError::generic_err("not implemented")));

        // let value: QueryTokenAddressResponse = from_binary(&res).unwrap();
        // assert_eq!(
        //     "terra1hpajld8zs93md8zrs6sfy42zl0khqpmr07muw0",
        //     value.token_address
        // );
    }
}
