// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Build script to emit custom cfg flags based on NCOLORS feature selection.
//!
//! This enables cleaner conditional compilation in tests and code:
//! - `#[cfg(ncolors_min_4)]` instead of `#[cfg(not(feature = "ncolors_3"))]`
//! - `#[cfg(ncolors_min_5)]` instead of `#[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]`
//! - `#[cfg(ncolors_eq_6)]` for the default/most common case

fn main() {
    // Declare the custom cfg names to avoid warnings
    println!("cargo:rustc-check-cfg=cfg(ncolors_min_4)");
    println!("cargo:rustc-check-cfg=cfg(ncolors_min_5)");
    println!("cargo:rustc-check-cfg=cfg(ncolors_eq_6)");

    // NCOLORS >= 4 (excludes only ncolors_3)
    #[cfg(not(feature = "ncolors_3"))]
    println!("cargo:rustc-cfg=ncolors_min_4");

    // NCOLORS >= 5 (excludes ncolors_3 and ncolors_4)
    #[cfg(not(any(feature = "ncolors_3", feature = "ncolors_4")))]
    println!("cargo:rustc-cfg=ncolors_min_5");

    // NCOLORS = 6 (default or explicit ncolors_6 feature)
    #[cfg(any(
        feature = "ncolors_6",
        not(any(feature = "ncolors_3", feature = "ncolors_4", feature = "ncolors_5"))
    ))]
    println!("cargo:rustc-cfg=ncolors_eq_6");
}
