pub mod address;
pub mod api_keys;
pub mod business_profile;
pub mod capture;
pub mod cards_info;
pub mod configs;

pub mod authorization;
pub mod customers;
pub mod dispute;
pub mod encryption;
pub mod enums;
pub mod ephemeral_key;
pub mod errors;
pub mod events;
pub mod file;
#[allow(unused)]
pub mod fraud_check;
pub mod gsm;
#[cfg(feature = "kv_store")]
pub mod kv;
pub mod locker_mock_up;
pub mod macros;
pub mod mandate;
pub mod merchant_account;
pub mod merchant_connector_account;
pub mod merchant_key_store;
pub mod organization;
pub mod payment_attempt;
pub mod payment_intent;
pub mod payment_link;
pub mod payment_method;
pub mod payout_attempt;
pub mod payouts;
pub mod process_tracker;
pub mod query;
pub mod refund;
pub mod reverse_lookup;
pub mod routing_algorithm;
#[allow(unused_qualifications)]
pub mod schema;
pub mod user;
pub mod user_role;

use diesel_impl::{DieselArray, OptionalDieselArray};

pub type StorageResult<T> = error_stack::Result<T, errors::DatabaseError>;
pub type PgPooledConn = async_bb8_diesel::Connection<diesel::PgConnection>;
pub use self::{
    address::*, api_keys::*, cards_info::*, configs::*, customers::*, dispute::*, ephemeral_key::*,
    events::*, file::*, locker_mock_up::*, mandate::*, merchant_account::*,
    merchant_connector_account::*, payment_attempt::*, payment_intent::*, payment_method::*,
    process_tracker::*, refund::*, reverse_lookup::*,
};

/// The types and implementations provided by this module are required for the schema generated by
/// `diesel_cli` 2.0 to work with the types defined in Rust code. This is because
/// [`diesel`][diesel] 2.0 [changed the nullability of array elements][diesel-2.0-array-nullability],
/// which causes [`diesel`][diesel] to deserialize arrays as `Vec<Option<T>>`. To prevent declaring
/// array elements as `Option<T>`, this module provides types and implementations to deserialize
/// arrays as `Vec<T>`, considering only non-null values (`Some(T)`) among the deserialized
/// `Option<T>` values.
///
/// [diesel-2.0-array-nullability]: https://diesel.rs/guides/migration_guide.html#2-0-0-nullability-of-array-elements

#[doc(hidden)]
pub(crate) mod diesel_impl {
    use diesel::{
        deserialize::FromSql,
        pg::Pg,
        sql_types::{Array, Nullable},
        Queryable,
    };

    pub struct DieselArray<T>(Vec<Option<T>>);

    impl<T> From<DieselArray<T>> for Vec<T> {
        fn from(array: DieselArray<T>) -> Self {
            array.0.into_iter().flatten().collect()
        }
    }

    impl<T, U> Queryable<Array<Nullable<U>>, Pg> for DieselArray<T>
    where
        T: FromSql<U, Pg>,
        Vec<Option<T>>: FromSql<Array<Nullable<U>>, Pg>,
    {
        type Row = Vec<Option<T>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }

    pub struct OptionalDieselArray<T>(Option<Vec<Option<T>>>);

    impl<T> From<OptionalDieselArray<T>> for Option<Vec<T>> {
        fn from(option_array: OptionalDieselArray<T>) -> Self {
            option_array
                .0
                .map(|array| array.into_iter().flatten().collect())
        }
    }

    impl<T, U> Queryable<Nullable<Array<Nullable<U>>>, Pg> for OptionalDieselArray<T>
    where
        Option<Vec<Option<T>>>: FromSql<Nullable<Array<Nullable<U>>>, Pg>,
    {
        type Row = Option<Vec<Option<T>>>;

        fn build(row: Self::Row) -> diesel::deserialize::Result<Self> {
            Ok(Self(row))
        }
    }
}

pub(crate) mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric, metrics_context, once_cell};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "ROUTER_API");

    counter_metric!(DATABASE_CALLS_COUNT, GLOBAL_METER);
    histogram_metric!(DATABASE_CALL_TIME, GLOBAL_METER);
}
