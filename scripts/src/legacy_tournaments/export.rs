use super::bundle::{validate, BundleManifest, EncodedBundle};
use anyhow::{ensure, Context, Result};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedBundle {
    pub output: PathBuf,
    pub manifest: PathBuf,
    pub details: BundleManifest,
}

pub fn manifest_path(output: &Path) -> PathBuf {
    let mut name = output.as_os_str().to_os_string();
    name.push(".manifest.json");
    PathBuf::from(name)
}

/// Publishes a complete bundle without ever overwriting an existing artifact.
///
/// Both payloads are prepared in private same-directory temporary files. The
/// data link is installed first and the manifest link last; only a bundle with
/// its manifest is accepted by the later importer.
pub fn publish(output: &Path, bundle: &EncodedBundle) -> Result<PublishedBundle> {
    validate(&bundle.jsonl, &bundle.manifest).context("refusing to publish an invalid bundle")?;

    let manifest = manifest_path(output);
    ensure!(
        !output.exists(),
        "output {} already exists",
        output.display()
    );
    ensure!(
        !manifest.exists(),
        "manifest {} already exists",
        manifest.display()
    );
    let parent = parent_directory(output);
    ensure!(
        parent.is_dir(),
        "output directory {} does not exist",
        parent.display()
    );

    let manifest_bytes = manifest_bytes(&bundle.manifest)?;
    let (data_temp, mut data_file) = create_private_temp(output, "jsonl")?;
    let (manifest_temp, mut manifest_file) = match create_private_temp(output, "manifest") {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&data_temp);
            return Err(error);
        }
    };

    let result = (|| -> Result<()> {
        write_private(&mut data_file, &bundle.jsonl, "JSONL")?;
        write_private(&mut manifest_file, &manifest_bytes, "manifest")?;
        drop(data_file);
        drop(manifest_file);

        fs::hard_link(&data_temp, output).with_context(|| {
            format!(
                "could not atomically publish legacy export {}",
                output.display()
            )
        })?;
        sync_directory(parent)?;
        ensure!(
            same_file(output, &data_temp),
            "legacy export output changed before its manifest could be published"
        );

        if let Err(error) = fs::hard_link(&manifest_temp, &manifest) {
            remove_if_same_file(output, &data_temp);
            return Err(error).with_context(|| {
                format!(
                    "could not atomically publish legacy export manifest {}",
                    manifest.display()
                )
            });
        }
        if !same_file(output, &data_temp) || !same_file(&manifest, &manifest_temp) {
            remove_if_same_file(&manifest, &manifest_temp);
            remove_if_same_file(output, &data_temp);
            anyhow::bail!("legacy export artifacts changed during atomic publication");
        }
        if let Err(error) = sync_directory(parent) {
            remove_if_same_file(&manifest, &manifest_temp);
            remove_if_same_file(output, &data_temp);
            return Err(error);
        }
        Ok(())
    })();

    let _ = fs::remove_file(&data_temp);
    let _ = fs::remove_file(&manifest_temp);
    result?;

    Ok(PublishedBundle {
        output: output.to_path_buf(),
        manifest,
        details: bundle.manifest.clone(),
    })
}

fn manifest_bytes(manifest: &BundleManifest) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(manifest)
        .context("could not encode the legacy export manifest")?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn create_private_temp(target: &Path, role: &str) -> Result<(PathBuf, File)> {
    let parent = parent_directory(target);
    let target_name = target
        .file_name()
        .context("legacy export output must name a file")?;
    for attempt in 0_u16..=u16::MAX {
        let mut name = OsString::from(".");
        name.push(target_name);
        name.push(format!(".{role}.{}.{attempt}.tmp", std::process::id()));
        let path = parent.join(name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not create private export file in {}",
                        parent.display()
                    )
                });
            }
        }
    }
    anyhow::bail!(
        "could not allocate a private export file in {}",
        parent.display()
    )
}

fn write_private(file: &mut File, bytes: &[u8], description: &str) -> Result<()> {
    file.write_all(bytes)
        .with_context(|| format!("could not write legacy export {description}"))?;
    file.sync_all()
        .with_context(|| format!("could not sync legacy export {description}"))?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .with_context(|| format!("could not sync export directory {}", path.display()))
}

fn parent_directory(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

fn remove_if_same_file(link: &Path, source: &Path) {
    if same_file(link, source) {
        let _ = fs::remove_file(link);
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    left.metadata()
        .and_then(|left_metadata| {
            right.metadata().map(|right_metadata| {
                left_metadata.dev() == right_metadata.dev()
                    && left_metadata.ino() == right_metadata.ino()
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy_tournaments::model::{
        AuditOutcome,
        AuditReport,
        LegacySource,
        LegacySourceMetadata,
    };
    use std::{collections::BTreeMap, os::unix::fs::PermissionsExt};

    fn bundle() -> EncodedBundle {
        let metadata = LegacySourceMetadata {
            database_name: "hive-local".to_string(),
        };
        let source = LegacySource {
            metadata: metadata.clone(),
            tournaments: Vec::new(),
            memberships: Vec::new(),
            organizers: Vec::new(),
            invitations: Vec::new(),
            games: Vec::new(),
            schedules: Vec::new(),
            series: Vec::new(),
            series_organizers: Vec::new(),
        };
        let audit = AuditOutcome {
            report: AuditReport {
                source: metadata,
                tournaments: Vec::new(),
                counts: BTreeMap::new(),
                diagnostics: Vec::new(),
            },
            plans: Vec::new(),
        };
        EncodedBundle::build(&source, &audit).unwrap()
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("hive-legacy-export-{name}-{}", std::process::id()));
        if directory.exists() {
            fs::remove_dir_all(&directory).unwrap();
        }
        fs::create_dir(&directory).unwrap();
        directory
    }

    #[test]
    fn publishes_private_data_then_acceptance_manifest_without_overwrite() {
        let directory = test_directory("publish");
        let output = directory.join("cutover.jsonl");
        let published = publish(&output, &bundle()).unwrap();

        assert_eq!(fs::read(&output).unwrap(), bundle().jsonl);
        let manifest: BundleManifest =
            serde_json::from_slice(&fs::read(&published.manifest).unwrap()).unwrap();
        assert_eq!(manifest, published.details);
        assert_eq!(
            output.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            published.manifest.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );

        let before = fs::read(&output).unwrap();
        assert!(publish(&output, &bundle()).is_err());
        assert_eq!(fs::read(&output).unwrap(), before);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_bundle_leaves_no_accepted_artifact() {
        let directory = test_directory("invalid");
        let output = directory.join("cutover.jsonl");
        let mut invalid = bundle();
        invalid.jsonl.push(b'x');
        assert!(publish(&output, &invalid).is_err());
        assert!(!output.exists());
        assert!(!manifest_path(&output).exists());
        fs::remove_dir_all(directory).unwrap();
    }
}
