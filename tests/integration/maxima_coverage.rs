//! Automated test verifying 1:1 Maxima LISP source citations for all rewrite rules.

use sym_core::rules::all_rule_provenances;

#[test]
fn test_all_rules_have_maxima_citations() {
    let rules = all_rule_provenances();
    assert!(!rules.is_empty(), "Rule registry should not be empty");

    let valid_source_files = [
        "simp.lisp",
        "trigi.lisp",
        "combin.lisp",
        "logarc.lisp",
        "sinh.lisp",
        "nrat4.lisp",
        "sfacts.lisp",
    ];

    let mut file_counts = std::collections::HashMap::new();

    for (name, prov) in &rules {
        assert!(!name.is_empty(), "Rule name must not be empty");
        assert!(
            valid_source_files.contains(&prov.source_file),
            "Rule '{name}' cites unknown Maxima file '{}'. Must be one of {:?}",
            prov.source_file,
            valid_source_files
        );
        assert!(
            !prov.source_fn.is_empty(),
            "Rule '{name}' must cite a specific Maxima source function"
        );
        assert!(
            !prov.identity.is_empty(),
            "Rule '{name}' must have an identity string"
        );

        *file_counts.entry(prov.source_file).or_insert(0) += 1;
    }

    // Verify all primary Maxima source files have active rules mapped
    for file in valid_source_files {
        assert!(file_counts.contains_key(file), "{file} must have rules");
    }

    // Phase 18: expanded 1:1 coverage must keep growing.
    assert!(
        rules.len() >= 92,
        "expected at least 92 provenanced rules, found {}",
        rules.len()
    );

    println!("\n=== Maxima Lisp 1:1 Rule Citation Coverage Summary ===");
    println!("Total rules mapped: {}", rules.len());
    for (file, count) in &file_counts {
        println!("  {:<15}: {} rules", file, count);
    }
    println!("=======================================================\n");
}
