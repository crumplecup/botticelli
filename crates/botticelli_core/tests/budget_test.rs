//! Budget configuration validation and calculation tests.

mod helpers;

use botticelli_core::{BudgetConfig, BudgetConfigBuilderError};

#[test]
fn validate_rejects_invalid_multipliers() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing budget validation with invalid multipliers");

    debug!(multiplier = 0.0, "Testing RPM multiplier = 0.0");
    let budget = BudgetConfig::builder().rpm_multiplier(0.0).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message().contains("RPM multiplier"));
    debug!("Correctly rejected multiplier = 0.0");

    debug!(multiplier = 1.5, "Testing RPM multiplier > 1.0");
    let budget = BudgetConfig::builder().rpm_multiplier(1.5).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message().contains("RPM multiplier"));
    debug!("Correctly rejected multiplier = 1.5");

    debug!(multiplier = -0.1, "Testing negative RPM multiplier");
    let budget = BudgetConfig::builder().rpm_multiplier(-0.1).build()?;
    let result = budget.validate();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.message().contains("RPM multiplier"));
    debug!("Correctly rejected multiplier = -0.1");

    info!("All invalid multiplier validations passed");
    Ok(())
}

#[test]
fn validate_accepts_valid_multipliers() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing budget validation with valid multipliers");
    debug!(rpm = 0.8, tpm = 0.5, rpd = 1.0, "Creating budget config");

    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(1.0)
        .build()?;

    assert!(budget.validate().is_ok());
    info!("Valid multipliers accepted successfully");
    Ok(())
}

#[test]
fn apply_methods_scale_correctly() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing budget scaling calculations");

    let budget = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.5)
        .rpd_multiplier(0.2)
        .build()?;

    debug!(input = 10, expected = 8, "Testing RPM scaling");
    assert_eq!(budget.apply_rpm(10), 8);

    debug!(input = 1000, expected = 500, "Testing TPM scaling");
    assert_eq!(budget.apply_tpm(1000), 500);

    debug!(input = 100, expected = 20, "Testing RPD scaling");
    assert_eq!(budget.apply_rpd(100), 20);

    info!("All scaling calculations correct");
    Ok(())
}

#[test]
fn merge_takes_minimum() -> anyhow::Result<()> {
    helpers::init_test_tracing("info");
    use tracing::{debug, info};

    info!("Testing budget merge operation");

    debug!("Creating first budget config");
    let budget1 = BudgetConfig::builder()
        .rpm_multiplier(0.8)
        .tpm_multiplier(0.9)
        .build()?;

    debug!("Creating second budget config");
    let budget2 = BudgetConfig::builder()
        .rpm_multiplier(0.5)
        .rpd_multiplier(0.3)
        .build()?;

    debug!("Merging budget configs");
    let merged = budget1.merge(&budget2);

    debug!(result = 0.5, expected = 0.5, "Checking RPM merge");
    assert_eq!(*merged.rpm_multiplier(), 0.5); // min(0.8, 0.5)

    debug!(result = 0.9, expected = 0.9, "Checking TPM merge");
    assert_eq!(*merged.tpm_multiplier(), 0.9); // min(0.9, 1.0)

    debug!(result = 0.3, expected = 0.3, "Checking RPD merge");
    assert_eq!(*merged.rpd_multiplier(), 0.3); // min(1.0, 0.3)

    info!("Merge operation correctly takes minimum values");
    Ok(())
}
