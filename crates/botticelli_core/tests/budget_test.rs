use botticelli_core::{BudgetConfig, BudgetConfigBuilderError};

#[test]
fn default_budget_uses_full_quota() {
    let budget = BudgetConfig::default();
    assert_eq!(*budget.rpm_multiplier(), 1.0);
    assert_eq!(*budget.tpm_multiplier(), 1.0);
    assert_eq!(*budget.rpd_multiplier(), 1.0);
}

#[test]
fn builder_works() -> Result<(), BudgetConfigBuilderError> {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .rpd_multiplier(0.5)
        .build()?;

    assert_eq!(*budget.rpm_multiplier(), 0.8);
    assert_eq!(*budget.tpm_multiplier(), 1.0); // Default
    assert_eq!(*budget.rpd_multiplier(), 0.5);
    Ok(())
}

#[test]
fn builder_returns_error_on_missing_required_fields() {
    // BudgetConfig has no required fields, all have defaults
    // This test verifies the builder always succeeds
    let result = BudgetConfig::builder().build();
    assert!(result.is_ok());
}

#[test]
fn validate_rejects_invalid_multipliers() -> Result<(), BudgetConfigBuilderError> {
    let budget = BudgetConfig::builder().rpm_multiplier(0.0).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("RPM multiplier"));

    let budget = BudgetConfig::builder().rpm_multiplier(1.5).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("RPM multiplier"));

    let budget = BudgetConfig::builder().rpm_multiplier(-0.1).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("RPM multiplier"));
    
    Ok(())
}

#[test]
fn validate_accepts_valid_multipliers() -> Result<(), BudgetConfigBuilderError> {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(1.0)
        .build()?;

    assert!(budget.validate().is_ok());
    Ok(())
}

#[test]
fn apply_methods_scale_correctly() -> Result<(), BudgetConfigBuilderError> {
    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(0.2)
        .build()?;

    assert_eq!(budget.apply_rpm(10), 8);
    assert_eq!(budget.apply_tpm(1000), 500);
    assert_eq!(budget.apply_rpd(100), 20);
    Ok(())
}

#[test]
fn merge_takes_minimum() -> Result<(), BudgetConfigBuilderError> {
    let budget1 = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.9)
        .build()?;

    let budget2 = BudgetConfig::builder()
        .rpm_multiplier(0.5)
        .rpd_multiplier(0.3)
        .build()?;

    let merged = budget1.merge(&budget2);

    assert_eq!(*merged.rpm_multiplier(), 0.5); // min(0.8, 0.5)
    assert_eq!(*merged.tpm_multiplier(), 0.9); // min(0.9, 1.0)
    assert_eq!(*merged.rpd_multiplier(), 0.3); // min(1.0, 0.3)
    Ok(())
}
