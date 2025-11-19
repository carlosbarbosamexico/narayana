pub mod rest;
pub mod grpc;
pub mod elegant;
pub mod dsl;
pub mod powerful;
pub mod websocket;
pub mod webhooks;
pub mod ultimate;
pub mod rest_advanced;
pub mod grpc_advanced;
pub mod query_dsl;
pub mod connection;
pub mod graphql;

pub use rest::*;
pub use grpc::*;
pub use connection::{Connection, DirectConnection, RemoteConnection};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod graphql_tests;
