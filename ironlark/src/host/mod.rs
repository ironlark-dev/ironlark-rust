//! The crate's side of the component boundary: the generated bindings, the
//! blanket impls the host enters, the dispatch tables for declared traffic,
//! the log bridge and the event scope. Built natively, the testing doubles
//! stand in for the host behind the same seams.

#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub mod bindings;
pub(crate) mod dispatch;
#[cfg(target_arch = "wasm32")]
pub(crate) mod glue;
pub(crate) mod log_bridge;
pub(crate) mod scope;
