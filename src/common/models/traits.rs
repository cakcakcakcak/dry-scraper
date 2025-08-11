use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::common::db::DbEntity;

// use crate::common::models::ItemParsedWithContext;

pub trait IntoDbStruct: HasTypeName + Debug + Serialize + Sized + for<'a> Deserialize<'a> {
    type DbStruct: DbStruct;
    type Context;

    fn to_db_struct(self, context: Self::Context) -> Self::DbStruct;

    // fn to_item_parsed_with_context(self, context: Self::Context) -> ItemParsedWithContext<Self> {
    //     ItemParsedWithContext {
    //         item: self,
    //         context: context,
    //     }
    // }
}

pub trait DbStruct: Debug + DbEntity + HasTypeName {
    type IntoDbStruct: IntoDbStruct;

    fn create_context_struct(&self) -> <<Self as DbStruct>::IntoDbStruct as IntoDbStruct>::Context;
}

pub trait HasTypeName {
    fn type_name() -> &'static str;
}
