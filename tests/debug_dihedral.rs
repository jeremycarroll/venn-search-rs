// Copyright (C) 2025 Jeremy J. Carroll. See LICENSE for details.

//! Debug test to print and verify dihedral groups

use venn_search::symmetry::*;

#[test]
fn test_print_dihedral_groups() {
    println!("\n=== Dihedral Group D_3 ===");
    for (i, perm) in DIHEDRAL_GROUP_3.iter().enumerate() {
        println!("  [{}]: {:?}", i, perm);
    }

    println!("\n=== Dihedral Group D_4 ===");
    for (i, perm) in DIHEDRAL_GROUP_4.iter().enumerate() {
        println!("  [{}]: {:?}", i, perm);
    }

    println!("\n=== Dihedral Group D_5 ===");
    for (i, perm) in DIHEDRAL_GROUP_5.iter().enumerate() {
        println!("  [{}]: {:?}", i, perm);
    }

    println!("\n=== Dihedral Group D_6 ===");
    for (i, perm) in DIHEDRAL_GROUP_6.iter().enumerate() {
        println!("  [{}]: {:?}", i, perm);
    }
}
