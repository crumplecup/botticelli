use botticelli_core::BudgetConfig;

#[test]
fn default_budget_uses_full_quota() {
    let budget = BudgetConfig::default();
    assert_eq!(*budget.rpm_multiplier(), 1.0);
    assert_eq!(*budget.tpm_multiplier(), 1.0);
    assert_eq!(*budget.rpd_multiplier(), 1.0);
}

#[test]
fn builder_works() {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .rpd_multiplier(0.5)
        .build();

    assert_eq!(*budget.rpm_multiplier(), 0.8);
    assert_eq!(*budget.tpm_multiplier(), 1.0); // Default
    assert_eq!(*budget.rpd_multiplier(), 0.5);
}

#[test]
fn validate_rejects_invalid_multipliers() {
    let budget = BudgetConfig::builder().rpm_multiplier(0.0).build();
    assert!(budget.validate().is_err());

    let budget = BudgetConfig::builder().rpm_multiplier(1.5).build();
    assert!(budget.validate().is_err());

    let budget = BudgetConfig::builder().rpm_multiplier(-0.1).build();
    assert!(budget.validate().is_err());
}

#[test]
fn validate_accepts_valid_multipliers() {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(1.0)
        .build();

    assert!(budget.validate().is_ok());
}

#[test]
fn apply_methods_scale_correctly() {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(0.2)
        .build();

    assert_eq!(budget.apply_rpm(10), 8);
    assert_eq!(budget.apply_tpm(1000), 500);
    assert_eq!(budget.apply_rpd(100), 20);
}

#[test]
fn merge_takes_minimum() {
    let budget1 = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.9)
        .build();

    let budget2 = BudgetConfig::builder()
        .rpm_multiplier(0.5)
        .rpd_multiplier(0.3)
        .build();

    let merged = budget1.merge(&budget2);

    assert_eq!(*merged.rpm_multiplier(), 0.5); // min(0.8, 0.5)
    assert_eq!(*merged.tpm_multiplier(), 0.9); // min(0.9, 1.0)
    assert_eq!(*merged.rpd_multiplier(), 0.3); // min(1.0, 0.3)
}
