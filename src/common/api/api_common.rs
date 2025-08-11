use std::fmt::Debug;

pub trait HasBaseUrl {
    fn base_url(&self) -> &str;
}

pub trait HasEndpoint {
    type Api: HasBaseUrl;
    type Params: Default + Debug;

    fn endpoint(api: &Self::Api, params: Self::Params) -> String;
}

pub trait FromId {
    fn from_id(id: i32) -> Self;
}
