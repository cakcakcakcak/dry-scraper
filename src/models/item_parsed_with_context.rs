use crate::models::traits::{DbStruct, IntoDbStruct};

pub struct ItemParsedWithContext<T> {
    pub item: T,
    pub endpoint: String,
    pub raw_data: String,
}
impl<T> ItemParsedWithContext<T>
where
    T: IntoDbStruct,
{
    pub fn to_db_struct(self) -> <T as IntoDbStruct>::U {
        let mut db_struct: <T as IntoDbStruct>::U = self.item.to_db_struct();
        db_struct.fill_context(self.endpoint, self.raw_data);
        db_struct
    }
}
