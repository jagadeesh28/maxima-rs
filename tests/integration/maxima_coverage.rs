//! Automated test verifying 1:1 Maxima LISP source citations for all rewrite rules.

use sym_core::rules::all_rule_provenances;

#[test]
fn test_all_rules_have_maxima_citations() {
    let rules = all_rule_provenances();
    assert!(!rules.is_empty(), "Rule registry should not be empty");

    let valid_source_files = ["simp.lisp", "trigi.lisp", "combin.lisp", "logarc.lisp"];

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

    // Verify all 4 primary Maxima source files have active rules mapped
    assert!(
        file_counts.contains_key("simp.lisp"),
        "simp.lisp must have rules"
    );
    assert!(
        file_counts.contains_key("trigi.lisp"),
        "trigi.lisp must have rules"
    );
    assert!(
        file_counts.contains_key("logarc.lisp"),
        "logarc.lisp must have rules"
    );
    assert!(
        file_counts.contains_key("combin.lisp"),
        "combin.lisp must have rules"
    );

    println!("\n=== Maxima Lisp 1:1 Rule Citation Coverage Summary ===");
    println!("Total rules mapped: {}", rules.len());
    for (file, count) in &file_counts {
        println!("  {:<15}: {} rules", file, count);
    }
    println!("=======================================================\n");
}
