use crate::common::models::ApiCacheKey;

use crate::data_sources::nhl::NhlPrimaryKey;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AnyPrimaryKey {
    ApiCache(ApiCacheKey),
    Nhl(NhlPrimaryKey),
}
