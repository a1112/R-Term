//! Isolates the actual app font boundary for compilation against frozen crates
//! before the independently implemented GPU adapter is available.
#![cfg(feature = "rterm-legacy-0-1")]
#![allow(
    dead_code,
    reason = "font-only harness does not run GUI service entrypoints"
)]

#[path = "../src/platform_fonts.rs"]
mod platform_fonts;
#[path = "../src/rterm_compat.rs"]
mod rterm_compat;
