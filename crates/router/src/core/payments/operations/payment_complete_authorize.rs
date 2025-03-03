use std::marker::PhantomData;

use api_models::enums::FrmSuggestion;
use async_trait::async_trait;
use error_stack::{report, IntoReport, ResultExt};
use router_derive::PaymentOperation;
use router_env::{instrument, tracing};

use super::{BoxedOperation, Domain, GetTracker, Operation, UpdateTracker, ValidateRequest};
use crate::{
    core::{
        errors::{self, CustomResult, RouterResult, StorageErrorExt},
        payment_methods::PaymentMethodRetrieve,
        payments::{self, helpers, operations, CustomerDetails, PaymentAddress, PaymentData},
        utils as core_utils,
    },
    db::StorageInterface,
    routes::AppState,
    services,
    types::{
        self,
        api::{self, PaymentIdTypeExt},
        domain,
        storage::{self, enums as storage_enums},
    },
    utils::{self, OptionExt},
};

#[derive(Debug, Clone, Copy, PaymentOperation)]
#[operation(operations = "all", flow = "authorize")]
pub struct CompleteAuthorize;

#[async_trait]
impl<F: Send + Clone, Ctx: PaymentMethodRetrieve>
    GetTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for CompleteAuthorize
{
    #[instrument(skip_all)]
    async fn get_trackers<'a>(
        &'a self,
        state: &'a AppState,
        payment_id: &api::PaymentIdType,
        request: &api::PaymentsRequest,
        mandate_type: Option<api::MandateTransactionType>,
        merchant_account: &domain::MerchantAccount,
        key_store: &domain::MerchantKeyStore,
        _auth_flow: services::AuthFlow,
    ) -> RouterResult<operations::GetTrackerResponse<'a, F, api::PaymentsRequest, Ctx>> {
        let db = &*state.store;
        let merchant_id = &merchant_account.merchant_id;
        let storage_scheme = merchant_account.storage_scheme;
        let (mut payment_intent, mut payment_attempt, currency, amount);

        let payment_id = payment_id
            .get_payment_intent_id()
            .change_context(errors::ApiErrorResponse::PaymentNotFound)?;

        payment_intent = db
            .find_payment_intent_by_payment_id_merchant_id(&payment_id, merchant_id, storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
        payment_intent.setup_future_usage = request
            .setup_future_usage
            .or(payment_intent.setup_future_usage);

        helpers::validate_payment_status_against_not_allowed_statuses(
            &payment_intent.status,
            &[
                storage_enums::IntentStatus::Failed,
                storage_enums::IntentStatus::Succeeded,
            ],
            "confirm",
        )?;

        let (
            token,
            payment_method,
            payment_method_type,
            setup_mandate,
            recurring_mandate_payment_data,
            mandate_connector,
        ) = helpers::get_token_pm_type_mandate_details(
            state,
            request,
            mandate_type.clone(),
            merchant_account,
            key_store,
        )
        .await?;

        let browser_info = request
            .browser_info
            .clone()
            .map(|x| utils::Encode::<types::BrowserInformation>::encode_to_value(&x))
            .transpose()
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "browser_info",
            })?;

        payment_attempt = db
            .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &payment_intent.payment_id,
                merchant_id,
                &payment_intent.active_attempt.get_id(),
                storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;

        let token = token.or_else(|| payment_attempt.payment_token.clone());

        if let Some(payment_method) = payment_method {
            let should_validate_pm_or_token_given =
                //this validation should happen if data was stored in the vault
                helpers::should_store_payment_method_data_in_vault(
                    &state.conf.temp_locker_enable_config,
                    payment_attempt.connector.clone(),
                    payment_method,
                );
            if should_validate_pm_or_token_given {
                helpers::validate_pm_or_token_given(
                    &request.payment_method,
                    &request.payment_method_data,
                    &request.payment_method_type,
                    &mandate_type,
                    &token,
                )?;
            }
        }

        payment_attempt.payment_method = payment_method.or(payment_attempt.payment_method);
        payment_attempt.browser_info = browser_info;
        payment_attempt.payment_method_type =
            payment_method_type.or(payment_attempt.payment_method_type);
        payment_attempt.payment_experience = request.payment_experience;
        currency = payment_attempt.currency.get_required_value("currency")?;
        amount = payment_attempt.amount.into();

        helpers::validate_customer_id_mandatory_cases(
            request.setup_future_usage.is_some(),
            &payment_intent
                .customer_id
                .clone()
                .or_else(|| request.customer_id.clone()),
        )?;

        let shipping_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.shipping.as_ref(),
            payment_intent.shipping_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;
        let billing_address = helpers::create_or_find_address_for_payment_by_request(
            db,
            request.billing.as_ref(),
            payment_intent.billing_address_id.as_deref(),
            merchant_id,
            payment_intent.customer_id.as_ref(),
            key_store,
            &payment_intent.payment_id,
            merchant_account.storage_scheme,
        )
        .await?;

        let redirect_response = request
            .feature_metadata
            .as_ref()
            .and_then(|fm| fm.redirect_response.clone());

        payment_intent.shipping_address_id = shipping_address.clone().map(|i| i.address_id);
        payment_intent.billing_address_id = billing_address.clone().map(|i| i.address_id);
        payment_intent.return_url = request.return_url.as_ref().map(|a| a.to_string());

        payment_intent.allowed_payment_method_types = request
            .get_allowed_payment_method_types_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting allowed_payment_types to Value")?
            .or(payment_intent.allowed_payment_method_types);

        payment_intent.connector_metadata = request
            .get_connector_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting connector_metadata to Value")?
            .or(payment_intent.connector_metadata);

        payment_intent.feature_metadata = request
            .get_feature_metadata_as_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error converting feature_metadata to Value")?
            .or(payment_intent.feature_metadata);

        payment_intent.metadata = request.metadata.clone().or(payment_intent.metadata);

        // The operation merges mandate data from both request and payment_attempt
        let setup_mandate = setup_mandate.map(Into::into);

        let profile_id = payment_intent
            .profile_id
            .as_ref()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("'profile_id' not set in payment intent")?;

        let business_profile = db
            .find_business_profile_by_profile_id(profile_id)
            .await
            .to_not_found_response(errors::ApiErrorResponse::BusinessProfileNotFound {
                id: profile_id.to_string(),
            })?;

        let payment_data = PaymentData {
            flow: PhantomData,
            payment_intent,
            payment_attempt,
            currency,
            amount,
            email: request.email.clone(),
            mandate_id: None,
            mandate_connector,
            setup_mandate,
            token,
            address: PaymentAddress {
                shipping: shipping_address.as_ref().map(|a| a.into()),
                billing: billing_address.as_ref().map(|a| a.into()),
            },
            confirm: request.confirm,
            payment_method_data: request.payment_method_data.clone(),
            force_sync: None,
            refunds: vec![],
            disputes: vec![],
            attempts: None,
            sessions_token: vec![],
            card_cvc: request.card_cvc.clone(),
            creds_identifier: None,
            pm_token: None,
            connector_customer_id: None,
            recurring_mandate_payment_data,
            ephemeral_key: None,
            multiple_capture_data: None,
            redirect_response,
            surcharge_details: None,
            frm_message: None,
            payment_link_data: None,
        };

        let customer_details = Some(CustomerDetails {
            customer_id: request.customer_id.clone(),
            name: request.name.clone(),
            email: request.email.clone(),
            phone: request.phone.clone(),
            phone_country_code: request.phone_country_code.clone(),
        });

        let get_trackers_response = operations::GetTrackerResponse {
            operation: Box::new(self),
            customer_details,
            payment_data,
            business_profile,
        };

        Ok(get_trackers_response)
    }
}

#[async_trait]
impl<F: Clone + Send, Ctx: PaymentMethodRetrieve> Domain<F, api::PaymentsRequest, Ctx>
    for CompleteAuthorize
{
    #[instrument(skip_all)]
    async fn get_or_create_customer_details<'a>(
        &'a self,
        db: &dyn StorageInterface,
        payment_data: &mut PaymentData<F>,
        request: Option<CustomerDetails>,
        key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<
        (
            BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
            Option<domain::Customer>,
        ),
        errors::StorageError,
    > {
        helpers::create_customer_if_not_exist(
            Box::new(self),
            db,
            payment_data,
            request,
            &key_store.merchant_id,
            key_store,
        )
        .await
    }

    #[instrument(skip_all)]
    async fn make_pm_data<'a>(
        &'a self,
        state: &'a AppState,
        payment_data: &mut PaymentData<F>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(
        BoxedOperation<'a, F, api::PaymentsRequest, Ctx>,
        Option<api::PaymentMethodData>,
    )> {
        let (op, payment_method_data) =
            helpers::make_pm_data(Box::new(self), state, payment_data, merchant_key_store).await?;
        Ok((op, payment_method_data))
    }

    #[instrument(skip_all)]
    async fn add_task_to_process_tracker<'a>(
        &'a self,
        _state: &'a AppState,
        _payment_attempt: &storage::PaymentAttempt,
        _requeue: bool,
        _schedule_time: Option<time::PrimitiveDateTime>,
    ) -> CustomResult<(), errors::ApiErrorResponse> {
        Ok(())
    }

    async fn get_connector<'a>(
        &'a self,
        _merchant_account: &domain::MerchantAccount,
        state: &AppState,
        request: &api::PaymentsRequest,
        _payment_intent: &storage::PaymentIntent,
        _key_store: &domain::MerchantKeyStore,
    ) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
        // Use a new connector in the confirm call or use the same one which was passed when
        // creating the payment or if none is passed then use the routing algorithm
        helpers::get_connector_default(state, request.routing.clone()).await
    }
}

#[async_trait]
impl<F: Clone, Ctx: PaymentMethodRetrieve>
    UpdateTracker<F, PaymentData<F>, api::PaymentsRequest, Ctx> for CompleteAuthorize
{
    #[instrument(skip_all)]
    async fn update_trackers<'b>(
        &'b self,
        _state: &'b AppState,
        payment_data: PaymentData<F>,
        _customer: Option<domain::Customer>,
        _storage_scheme: storage_enums::MerchantStorageScheme,
        _updated_customer: Option<storage::CustomerUpdate>,
        _merchant_key_store: &domain::MerchantKeyStore,
        _frm_suggestion: Option<FrmSuggestion>,
        _header_payload: api::HeaderPayload,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        PaymentData<F>,
    )>
    where
        F: 'b + Send,
    {
        Ok((Box::new(self), payment_data))
    }
}

impl<F: Send + Clone, Ctx: PaymentMethodRetrieve> ValidateRequest<F, api::PaymentsRequest, Ctx>
    for CompleteAuthorize
{
    #[instrument(skip_all)]
    fn validate_request<'a, 'b>(
        &'b self,
        request: &api::PaymentsRequest,
        merchant_account: &'a domain::MerchantAccount,
    ) -> RouterResult<(
        BoxedOperation<'b, F, api::PaymentsRequest, Ctx>,
        operations::ValidateResult<'a>,
    )> {
        let payment_id = request
            .payment_id
            .clone()
            .ok_or(report!(errors::ApiErrorResponse::PaymentNotFound))?;

        let request_merchant_id = request.merchant_id.as_deref();
        helpers::validate_merchant_id(&merchant_account.merchant_id, request_merchant_id)
            .change_context(errors::ApiErrorResponse::InvalidDataFormat {
                field_name: "merchant_id".to_string(),
                expected_format: "merchant_id from merchant account".to_string(),
            })?;

        helpers::validate_payment_method_fields_present(request)?;

        let mandate_type =
            helpers::validate_mandate(request, payments::is_operation_confirm(self))?;

        Ok((
            Box::new(self),
            operations::ValidateResult {
                merchant_id: &merchant_account.merchant_id,
                payment_id: payment_id
                    .and_then(|id| core_utils::validate_id(id, "payment_id"))
                    .into_report()?,
                mandate_type,
                storage_scheme: merchant_account.storage_scheme,
                requeue: matches!(
                    request.retry_action,
                    Some(api_models::enums::RetryAction::Requeue)
                ),
            },
        ))
    }
}
