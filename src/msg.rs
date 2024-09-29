use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub limit: Uint128,
}

#[cw_serde]
pub enum ExecuteMsg {
    IncrementLimit {},
    UpdateLimit { limit: Uint128 },
    UpdateLimitWithoutCheck { limit: Uint128 },
    SendTokens { recipient: String, amount: Uint128 },
    SendTokenToContract { amount: Uint128 },
    TransferTokens { sender: String, recipient: String, amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetLimitResponse)]
    GetLimit {},
    #[returns(GetValidatorResponse)]
    GetValidator {},

}

#[cw_serde]
pub enum SudoMsg {
    SendTokenToContract { amount: Uint128 },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct GetLimitResponse {
    pub limit: Uint128,
}

#[cw_serde]
pub struct GetValidatorResponse {
    pub validator: String,
}

