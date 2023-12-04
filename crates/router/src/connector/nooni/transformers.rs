use api_models::payments::Card;
use serde::{Deserialize, Serialize};
use cards::CardNumber;
use masking::Secret;
use crate::{connector::utils::{self, PaymentsAuthorizeRequestData, RouterData},core::errors,types::{self,api, storage::enums::{self, Currency}}};


// Auth Struct
pub struct NooniAuthType {
    pub(super) test_key: Secret<String>,
}

impl TryFrom<&types::ConnectorAuthType> for NooniAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                test_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}


#[derive(Debug, Serialize)]
pub struct NooniRouterData<T> {
    pub amount: i64,
    pub router_data: T,
}
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for NooniRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (_currency_unit, _currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NooniAuthorizeRequestSourceType {
Card,
GooglePay
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeRequestSource {
    #[serde(rename = "type")]
    pub nooni_authorize_request_source_type: NooniAuthorizeRequestSourceType,
    pub number: CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub name: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeRequest {
    pub source: NooniAuthorizeRequestSource,
    pub processing_channel_id: String,
    pub amount: i64,
    pub currency: diesel_models::enums::Currency,
    pub reference: Option<String>,
    pub capture: bool,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseBalances {
    pub total_authorized: i64,
    pub total_voided: i64,
    pub available_to_void: i64,
    pub total_captured: i64,
    pub available_to_capture: i64,
    pub total_refunded: i64,
    pub available_to_refund: i64,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseRisk {
    pub flagged: bool,
    pub score: i64,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseSource {
    pub id: String,
    #[serde(rename = "type")]
    pub nooni_authorize_response_source_type: String,
    pub expiry_month: i64,
    pub expiry_year: i64,
    pub name: String,
    pub scheme: String,
    pub last4: String,
    pub fingerprint: String,
    pub bin: String,
    pub card_type: String,
    pub card_category: String,
    pub issuer_country: String,
    pub product_id: String,
    pub product_type: String,
    pub avs_check: String,
    pub cvv_check: String,
    pub payment_account_reference: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseProcessing {
    pub acquirer_transaction_id: String,
    pub retrieval_reference_number: String,
    pub merchant_category_code: String,
    pub scheme_merchant_id: String,
    pub aft: bool,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseLinksSelf {
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseLinksActions {
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseLinksCapture {
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseLinksVoid {
    pub href: String,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponseLinks {
    #[serde(rename = "self")]
    pub nooni_authorize_response_links_self: NooniAuthorizeResponseLinksSelf,
    pub actions: NooniAuthorizeResponseLinksActions,
    pub capture: NooniAuthorizeResponseLinksCapture,
    pub void: NooniAuthorizeResponseLinksVoid,
}

#[derive(Debug, Serialize, Deserialize)]

pub struct NooniAuthorizeResponse {
    pub id: String,
    pub action_id: String,
    pub amount: i64,
    pub currency: String,
    pub approved: bool,
    pub status: NooniAttemptStatus,
    pub auth_code: String,
    pub response_code: String,
    pub response_summary: String,
    pub balances: NooniAuthorizeResponseBalances,
    pub risk: NooniAuthorizeResponseRisk,
    pub source: NooniAuthorizeResponseSource,
    pub processed_on: String,
    pub reference: String,
    pub scheme_id: String,
    pub processing: NooniAuthorizeResponseProcessing,
    pub expires_on: String,
    pub _links: NooniAuthorizeResponseLinks,
}

impl TryFrom<(&NooniRouterData<&types::PaymentsAuthorizeRouterData>, &Card)> for NooniAuthorizeRequest {
            type Error = error_stack::Report<errors::ConnectorError>;
            fn try_from(value: (&NooniRouterData<&types::PaymentsAuthorizeRouterData>, &Card)) -> Result<Self, Self::Error> {
                let (item, ccard) = value;
                let nooni_authorize_request_source_type = NooniAuthorizeRequestSourceType::Card;
			let nooni_authorize_request_source = NooniAuthorizeRequestSource{nooni_authorize_request_source_type:nooni_authorize_request_source_type,number:ccard.card_number.clone(),expiry_month:ccard.card_exp_month.clone(),expiry_year:ccard.card_exp_year.clone(),name:ccard.card_holder_name.clone(),cvv:ccard.card_cvc.clone()};
			let nooni_authorize_request = NooniAuthorizeRequest{source:nooni_authorize_request_source,processing_channel_id:"pc_gcjstkyrr4eudnjkqlro3kymcu".to_string(),amount:item.amount,currency:item.router_data.request.currency,reference:Some(item.router_data.connector_request_reference_id.clone()),capture:item.router_data.request.is_auto_capture()?};
                Ok(nooni_authorize_request)
            }
        }    
impl TryFrom<&NooniRouterData<&types::PaymentsAuthorizeRouterData>> for NooniAuthorizeRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &NooniRouterData<&types::PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
        match &item.router_data.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(card) => Self::try_from((item, card)),
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            ))?,
        }
    }
}

impl TryFrom<types::PaymentsResponseRouterData<NooniAuthorizeResponse>> 
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::PaymentsResponseRouterData<NooniAuthorizeResponse>,
    ) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data:  None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
            }),
            ..item.data
        })
    }
}
#[derive(Debug, Serialize, Deserialize)]

pub enum NooniAttemptStatus {
    Authorized
}
impl From<NooniAttemptStatus> for enums::AttemptStatus {
    fn from(item: NooniAttemptStatus) -> Self {
        match item {
            NooniAttemptStatus::Authorized => Self::Authorized
        }
    }
}


//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct RefundRequest {
    pub amount: i64
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self,Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>> for types::RefundsRouterData<api::RSync>
{
     type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<api::RSync, RefundResponse>) -> Result<Self,Self::Error> {
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
     }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}