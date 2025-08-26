use std::fmt::Debug;

use crate::{
    common::models::traits::{DbStruct, IntoDbStruct},
    with_progress_bar,
};

#[derive(Clone, Debug)]
pub struct ItemParsedWithContext<T: IntoDbStruct> {
    pub item: T,
    pub context: T::Context,
}
impl<T> ItemParsedWithContext<T>
where
    T: IntoDbStruct + Debug,
{
    pub fn into_db_struct(self) -> <T as IntoDbStruct>::DbStruct {
        let db_struct: <T as IntoDbStruct>::DbStruct = self.item.into_db_struct(self.context);
        db_struct
    }
}
pub trait ItemParsedWithContextVecExt<J>
where
    J: IntoDbStruct,
    J::DbStruct: DbStruct,
{
    fn into_db_structs(self) -> Vec<J::DbStruct>;
}

impl<J> ItemParsedWithContextVecExt<J> for Vec<ItemParsedWithContext<J>>
where
    J: IntoDbStruct,
    J::DbStruct: DbStruct,
{
    fn into_db_structs(self) -> Vec<J::DbStruct> {
        with_progress_bar!(self.len(), |pb| {
            self.into_iter()
                .map(|game_json| game_json.into_db_struct())
                .inspect(|_| pb.inc(1))
                .collect()
        })
    }
}
