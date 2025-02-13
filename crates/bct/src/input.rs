use rmx::prelude::*;

#[salsa::input]
pub struct Source {
    #[return_ref]
    pub text: String,
}
