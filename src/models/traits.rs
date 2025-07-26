use sqlx::FromRow;

pub trait IntoDbStruct {
    type DbStruct: DbStruct;
    type Context;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct;
}

pub trait DbStruct: for<'a> FromRow<'a, sqlx::postgres::PgRow>{}

pub trait HasTypeName {
    fn type_name() -> &'static str;
}
