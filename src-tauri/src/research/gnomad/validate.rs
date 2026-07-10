// ./src-tauri/src/research/gnomad/validate.rs
use super::config::{index_urls, normalize_chrom_for_template, vcf_url};
use super::types::{GnomadConfig, GnomadSourceTestResult};
use super::vcf_remote::head_url_ok;
use std::path::{Component, Path, PathBuf};

pub fn validate_gnomad_template(template: &str) -> Result<(), String> {
    let t = template.trim();
    if t.is_empty() || t.len() > 512 {
        return Err("gnomAD template must be 1–512 characters".into());
    }
    if t.contains("..") {
        return Err("gnomAD template must not contain '..'".into());
    }
    if t.starts_with('/') || t.starts_with('\\') {
        return Err("gnomAD template must be relative".into());
    }
    if t.contains(':') {
        return Err("gnomAD template must not contain ':'".into());
    }
    Ok(())
}

pub fn resolve_local_vcf_path(dir: &str, template: &str, chrom: &str) -> Result<PathBuf, String> {
    validate_gnomad_template(template)?;
    let dir_path = Path::new(dir.trim());
    if !dir_path.is_dir() {
        return Err(format!("gnomAD local VCF directory does not exist: {dir}"));
    }
    let dir_canon = std::fs::canonicalize(dir_path)
        .map_err(|e| format!("Invalid gnomAD local VCF directory: {e}"))?;
    let chrom_key = normalize_chrom_for_template(chrom);
    let relative = template.replace("{chrom}", &chrom_key);
    for component in Path::new(&relative).components() {
        match component {
            Component::Normal(_) => {}
            Component::ParentDir => {
                return Err("gnomAD template path must not traverse upward".into());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("gnomAD template must be relative to the local directory".into());
            }
            _ => {}
        }
    }
    let joined = dir_canon.join(&relative);
    if !joined.starts_with(&dir_canon) {
        return Err("Resolved gnomAD VCF path escapes the configured directory".into());
    }
    Ok(joined)
}

pub fn validate_gnomad_config_paths(cfg: &GnomadConfig) -> Result<(), String> {
    validate_gnomad_template(&cfg.exome_template)?;
    validate_gnomad_template(&cfg.genome_template)?;
    if let Some(ref dir) = cfg.local_vcf_dir
        && !dir.trim().is_empty()
    {
        resolve_local_vcf_path(dir, &cfg.exome_template, "22")?;
        resolve_local_vcf_path(dir, &cfg.genome_template, "22")?;
    }
    Ok(())
}

pub async fn test_gnomad_source_urls(cfg: &GnomadConfig) -> GnomadSourceTestResult {
    let chrom = "22";
    let exome_vcf_url = vcf_url(cfg, &cfg.exome_template, chrom);
    let genome_vcf_url = vcf_url(cfg, &cfg.genome_template, chrom);
    let (exome_tbi, exome_csi) = index_urls(&exome_vcf_url);
    let (genome_tbi, genome_csi) = index_urls(&genome_vcf_url);

    let exome_vcf_ok = head_url_ok(&exome_vcf_url).await;
    let exome_index_ok = head_url_ok(&exome_tbi).await || head_url_ok(&exome_csi).await;
    let genome_vcf_ok = head_url_ok(&genome_vcf_url).await;
    let genome_index_ok = head_url_ok(&genome_tbi).await || head_url_ok(&genome_csi).await;

    let mut errors = Vec::new();
    if !exome_vcf_ok {
        errors.push(format!("Exome VCF not reachable: {exome_vcf_url}"));
    }
    if !exome_index_ok {
        errors.push(format!("Exome index not reachable: {exome_tbi}"));
    }
    if !genome_vcf_ok {
        errors.push(format!("Genome VCF not reachable: {genome_vcf_url}"));
    }
    if !genome_index_ok {
        errors.push(format!("Genome index not reachable: {genome_tbi}"));
    }

    let smoke_test_ok = exome_vcf_ok && exome_index_ok && genome_vcf_ok && genome_index_ok;
    let smoke_test_message = if smoke_test_ok {
        Some(format!(
            "URLs verified for {} chr{} ({} provider).",
            cfg.release,
            chrom,
            match cfg.provider {
                super::types::GnomadHttpsProvider::Aws => "AWS HTTPS",
                super::types::GnomadHttpsProvider::Google => "Google HTTPS",
            }
        ))
    } else {
        None
    };

    GnomadSourceTestResult {
        release: cfg.release.clone(),
        provider: match cfg.provider {
            super::types::GnomadHttpsProvider::Aws => "aws".into(),
            super::types::GnomadHttpsProvider::Google => "google".into(),
        },
        exome_vcf_url,
        exome_index_url: exome_tbi,
        genome_vcf_url,
        genome_index_url: genome_tbi,
        exome_vcf_ok,
        exome_index_ok,
        genome_vcf_ok,
        genome_index_ok,
        smoke_test_ok,
        smoke_test_message,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn gnomad_template_rejects_traversal() {
        assert!(validate_gnomad_template("release/{chrom}.vcf.bgz").is_ok());
        assert!(validate_gnomad_template("../escape/{chrom}.vcf.bgz").is_err());
        assert!(validate_gnomad_template("/abs/{chrom}.vcf.bgz").is_err());
    }

    #[test]
    fn resolve_local_vcf_path_stays_under_directory() {
        let dir = std::env::temp_dir().join(format!("gnomad_path_test_{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let dir_str = dir.to_string_lossy().to_string();
        let template = "release/gnomad.exomes.chr{chrom}.vcf.bgz";

        let dir_canon = std::fs::canonicalize(&dir).expect("canonical dir");
        let resolved = resolve_local_vcf_path(&dir_str, template, "22").expect("resolve");
        assert!(resolved.starts_with(&dir_canon));

        assert!(resolve_local_vcf_path(&dir_str, "../outside/chr{chrom}.vcf.bgz", "22").is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
