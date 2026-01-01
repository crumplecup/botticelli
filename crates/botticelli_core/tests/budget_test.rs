use botticelli_core::{BudgetConfig, BudgetConfigBuilderError};

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
