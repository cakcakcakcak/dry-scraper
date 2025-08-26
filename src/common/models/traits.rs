use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::common::db::DbEntity;

pub trait IntoDbStruct: HasTypeName + Debug + Serialize + Sized + for<'a> Deserialize<'a> {
    type DbStruct: DbStruct;
    type Context;

    fn into_db_struct(self, context: Self::Context) -> Self::DbStruct;
}

pub trait DbStruct: Debug + DbEntity + HasTypeName {
    type IntoDbStruct: IntoDbStruct;
}

pub trait HasTypeName {
    fn type_name() -> &'static str;
}
