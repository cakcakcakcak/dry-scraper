use std::fmt::Debug;

use crate::common::models::traits::IntoDbStruct;

#[derive(Debug)]
pub struct ItemParsedWithContext<T: IntoDbStruct> {
    pub item: T,
    pub context: T::Context,
}
impl<T> ItemParsedWithContext<T>
where
    T: IntoDbStruct + Debug,
{
    pub fn to_db_struct(self) -> <T as IntoDbStruct>::DbStruct {
        let db_struct: <T as IntoDbStruct>::DbStruct = self.item.to_db_struct(self.context);
        db_struct
    }
}
