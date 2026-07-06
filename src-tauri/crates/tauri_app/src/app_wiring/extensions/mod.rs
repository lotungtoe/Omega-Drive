pub mod adapters;
pub mod context;
pub mod contract;
pub mod manifest;
pub mod ports;
pub mod registry;

#[allow(clippy::all)]
pub(crate) mod generated {
    include!(concat!(env!("OUT_DIR"), "/extensions_registry.g.rs"));
}
