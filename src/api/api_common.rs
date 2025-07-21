pub trait ApiContext {
    fn base_url(&self) -> &str;
}

pub trait HasEndpoint {
    type Params: Default + std::fmt::Debug;

    fn endpoint<A: ApiContext>(api: &A, params: Self::Params) -> String;
}

pub trait FromId {
    fn from_id(id: i32) -> Self;
}
