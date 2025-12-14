// Submodules
pub mod models;
pub mod client;
pub mod auth;
pub mod items;
pub mod folders;
pub mod collections;

// Re-exports
pub use client::VaultwardenClient;
pub use models::*;
// Methods are impl'd on VaultwardenClient in submodules, so just importing them makes them available if VaultwardenClient is used?
// No, extension traits or inherent impls in same crate.
// In Rust, if we put `impl VaultwardenClient` in different files in the same crate, 
// they are all part of the struct definition as long as we compile them.
// We just need to make sure the modules are declared.
