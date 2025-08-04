pub trait ApiContext {
    fn base_url(&self) -> &str;
}

pub trait HasEndpoint {
    type Api: ApiContext;
    type Params: Default + std::fmt::Debug;

    fn endpoint(api: &Self::Api, params: Self::Params) -> String;
}

pub trait FromId {
    fn from_id(id: i32) -> Self;
}
