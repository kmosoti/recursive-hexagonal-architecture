//! The L0 commands in `.rha/policy.toml` must equal the spec's §12.1 command
//! block, in order. §12.1 owns lane membership; the policy owns parameters.

use std::path::Path;

use xtask::policy;

#[test]
fn l0_argv_lists_equal_the_spec_block() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let loaded = policy::load(root).unwrap();
    let lane = &loaded.policy.lanes["L0"];
    let spec_path = root.join(lane.source_path());
    let spec = std::fs::read_to_string(&spec_path).unwrap();
    let spec_argv = policy::spec_l0_commands(&spec).unwrap();
    let policy_argv: Vec<Vec<String>> = lane.checks.iter().map(|c| c.argv.clone()).collect();

    let differences = policy::drift(&policy_argv, &spec_argv);
    assert!(
        differences.is_empty(),
        "policy L0 drifted from {}:\n{}",
        spec_path.display(),
        differences.join("\n")
    );
    assert_eq!(policy_argv.len(), 8, "§12.1 lists eight L0 commands");
}

#[test]
fn every_l0_check_is_required_and_non_waivable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let loaded = policy::load(root).unwrap();
    assert!(policy::validate(&loaded.policy).is_empty());
    for check in &loaded.policy.lanes["L0"].checks {
        assert!(check.required, "{} must be required", check.id);
        assert!(
            !check.waivable,
            "{} must be non-waivable (DP-0.5)",
            check.id
        );
    }
}
