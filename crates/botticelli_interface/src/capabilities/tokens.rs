//! Token counting capability.

use crate::BotticelliDriver;

/// Trait for backends that support token counting.
pub trait TokenCounting: BotticelliDriver {
    /// Count tokens in the given text.
    fn count_tokens(&self, text: &str) -> Result<usize, Self::Error>;

    /// Count tokens in a request before sending to the API.
    fn count_request_tokens(&self, req: &Self::Request) -> Result<usize, Self::Error>;
}
