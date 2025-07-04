use rmx::prelude::*;

#[salsa::input]
pub struct Source {
    #[returns(ref)]
    pub text: String,
}
