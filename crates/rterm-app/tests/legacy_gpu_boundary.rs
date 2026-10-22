//! Real app-owned frame and catalog-retirement adapter, isolated from GUI services.
#![cfg(feature = "rterm-legacy-0-1")]
#![allow(dead_code, reason = "GPU boundary harness does not run GUI services")]

#[path = "../src/platform_fonts.rs"]
mod platform_fonts;
#[path = "../src/rterm_compat.rs"]
mod rterm_compat;
#[path = "../src/rterm_compat_gpu.rs"]
mod rterm_compat_gpu;
