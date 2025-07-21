use crate::db::DbPool;
use crate::lp_error::LPError;

pub trait IntoDbStruct {
    type U: DbStruct;

    fn to_db_struct(self) -> Self::U;
}

pub trait DbStruct {
    fn fill_context(&mut self, endpoint: String, raw_data: String) -> Result<(), LPError>;
}

pub trait HasTypeName {
    fn type_name() -> &'static str;
}
