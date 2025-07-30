use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub trait IntoDbStruct: std::fmt::Debug + Serialize + for<'a> Deserialize<'a> {
    type DbStruct: DbStruct;
    type Context;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct;
}

pub trait DbStruct: Clone + std::fmt::Debug + for<'a> FromRow<'a, sqlx::postgres::PgRow> {
    type IntoDbStruct: IntoDbStruct;
}

pub trait HasTypeName {
    fn type_name() -> &'static str;
}
