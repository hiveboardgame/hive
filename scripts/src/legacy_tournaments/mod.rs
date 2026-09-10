mod audit;
pub mod bundle;
mod double_swiss;
pub mod export;
pub mod import;
mod legacy_standings;
mod model;
mod round_robin;
mod source;

use anyhow::{bail, ensure, Context, Result};
use bundle::EncodedBundle;
use db_lib::DbConn;
use export::PublishedBundle;
use model::{AuditOutcome, AuditSeverity, LegacySource};
use std::{io::Write, path::Path};

pub async fn inspect(conn: &mut DbConn<'_>) -> Result<(LegacySource, AuditOutcome)> {
    let source = source::load(conn).await?;
    let outcome = audit::run(&source);
    Ok((source, outcome))
}

pub async fn run_audit_command(conn: &mut DbConn<'_>) -> Result<()> {
    let (_, outcome) = inspect(conn).await?;
    write_report(&outcome)?;
    require_clean(&outcome)
}

pub async fn run_export_command(
    conn: &mut DbConn<'_>,
    output: &Path,
    confirmed_database_name: &str,
) -> Result<PublishedBundle> {
    let (source, outcome) = inspect(conn).await?;
    validate_database_confirmation(&source.metadata.database_name, confirmed_database_name)?;
    write_report(&outcome)?;
    require_clean(&outcome)?;
    let encoded = EncodedBundle::build(&source, &outcome)
        .context("the audited legacy source could not be encoded")?;
    export::publish(output, &encoded)
}

pub fn write_report(outcome: &AuditOutcome) -> Result<()> {
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer_pretty(&mut output, &outcome.report)
        .context("could not write the legacy tournament audit report")?;
    output
        .write_all(b"\n")
        .context("could not finish the legacy tournament audit report")
}

fn require_clean(outcome: &AuditOutcome) -> Result<()> {
    let failures = outcome
        .report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == AuditSeverity::HardFailure)
        .count();
    if failures != 0 {
        bail!("legacy tournament audit found {failures} hard failure(s)");
    }
    ensure!(
        outcome.plans.len() == outcome.report.tournaments.len(),
        "legacy tournament audit did not map every tournament"
    );
    Ok(())
}

fn validate_database_confirmation(actual: &str, confirmed: &str) -> Result<()> {
    ensure!(
        !confirmed.is_empty(),
        "the confirmed database name cannot be empty"
    );
    ensure!(
        actual == confirmed,
        "database confirmation mismatch: connected to {actual:?}, confirmed {confirmed:?}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_database_confirmation;

    #[test]
    fn database_confirmation_is_exact_and_permits_production_name() {
        assert!(validate_database_confirmation("hive-local", "hive-local").is_ok());
        assert!(validate_database_confirmation("hive-local", "").is_err());
        assert!(validate_database_confirmation("hive-local", "hive_local").is_err());
        assert!(validate_database_confirmation("hive-local", "hive-local ").is_err());
    }
}
