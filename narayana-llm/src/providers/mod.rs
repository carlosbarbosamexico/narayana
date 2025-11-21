pub mod trait_impl;
pub mod openai;
pub mod anthropic;
pub mod google;
pub mod cohere;

pub use trait_impl::Provider as ProviderTrait;
pub use crate::config::Provider;



