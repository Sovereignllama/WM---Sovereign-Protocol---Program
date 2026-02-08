pub mod initialize_protocol;
pub mod create_sovereign;
pub mod create_token;
pub mod transfer_hook;
pub mod deposit;
pub mod withdraw;
pub mod finalize;
pub mod claim_fees;
pub mod governance;
pub mod activity_check;
pub mod failed_bonding;
pub mod admin;
pub mod emergency;

// Glob re-exports for Anchor compatibility
// Note: "ambiguous glob re-exports" warning for `handler` is benign -
// lib.rs uses fully qualified paths (e.g., instructions::deposit::handler)
#[allow(ambiguous_glob_reexports)]
pub use initialize_protocol::*;
pub use create_sovereign::*;
pub use create_token::*;
pub use transfer_hook::*;
pub use deposit::*;
pub use withdraw::*;
pub use finalize::*;
pub use claim_fees::*;
pub use governance::*;
pub use activity_check::*;
pub use failed_bonding::*;
pub use admin::*;
pub use emergency::*;
