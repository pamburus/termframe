//! Build script for termframe
//!
//! This script performs the following tasks:
//! - Automatically replaces HTTP(S) schema URLs in TOML files with relative paths
//! - Updates theme aliases to their default values
//!
//! ## Schema URL Replacement
//!
//! Any TOML file in the assets directory that contains a schema directive like:
//!   #:schema https://example.com/path/to/schema.json
//!
//! Will be automatically replaced with a relative path to the local schema file:
//!   #:schema ../../schema/json/schema.json
//!
//! This replacement only occurs if:
//! - The schema URL uses HTTP(S) protocol
//! - A matching local schema file exists in schema/json/
//! - The local schema file has a different SHA256 hash than the remote one
//!   (or the remote fetch fails)

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use ureq::tls;

const ASSETS_DIR: &str = "assets";
const JSON_SCHEMA_DIR: &str = "schema/json";
const MAX_FETCH_ATTEMPTS: u32 = 3;
const BASE_FETCH_TIMEOUT: Duration = Duration::from_secs(2);

fn main() {
    if let Err(e) = run() {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    update_schema_directives()?;

    if let Err(e) = tidy_themes::update_theme_aliases_default() {
        return Err(anyhow!("Failed to update theme aliases: {}", e));
    }

    println!("cargo:rerun-if-changed=assets/themes");
    Ok(())
}

/// Updates schema directives in all TOML files under the assets directory.
/// Recursively processes all subdirectories.
fn update_schema_directives() -> Result<()> {
    // Build middleware chain: cache wraps retry wraps fetch
    let fetch_hash = with_cache(with_retry(fetch_and_hash_url, MAX_FETCH_ATTEMPTS));

    update_toml_schema_urls_in_dir(Path::new(ASSETS_DIR), &fetch_hash)?;
    Ok(())
}

/// Recursively processes a directory, updating schema URLs in all TOML files.
fn update_toml_schema_urls_in_dir(
    dir: &Path,
    fetch_hash: &impl Fn(&str) -> Result<Hash>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| anyhow!("Failed to read directory {}: {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            update_toml_schema_urls_in_dir(&path, fetch_hash)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            println!("cargo:rerun-if-changed={}", path.display());
            update_toml_schema_url(&path, fetch_hash)?;
        }
    }
    Ok(())
}

/// Updates the schema URL in a single TOML file if it uses HTTP(S) protocol.
/// Replaces HTTP(S) URLs with relative paths to local schema files.
fn update_toml_schema_url(
    toml_path: &Path,
    fetch_hash: &impl Fn(&str) -> Result<Hash>,
) -> Result<()> {
    const SCHEMA_PREFIX: &str = "#:schema ";

    let content = fs::read_to_string(toml_path)
        .map_err(|e| anyhow!("Failed to read TOML file {}: {}", toml_path.display(), e))?;

    let schema_line = content
        .lines()
        .find(|line| line.trim().starts_with(SCHEMA_PREFIX));

    let Some(schema_line) = schema_line else {
        return Ok(());
    };

    let schema_url = schema_line
        .trim()
        .strip_prefix(SCHEMA_PREFIX)
        .ok_or_else(|| anyhow!("Invalid schema directive in {}", toml_path.display()))?
        .trim();

    if !schema_url.starts_with("http://") && !schema_url.starts_with("https://") {
        return Ok(());
    }

    let schema_filename = schema_url
        .rsplit('/')
        .next()
        .ok_or_else(|| anyhow!("Invalid schema URL: {}", schema_url))?;

    let local_schema_path = find_local_schema_file(schema_filename)?;

    // Fetch remote hash with caching and retry
    let remote_hash = match fetch_hash(schema_url) {
        Ok(hash) => hash,
        Err(e) => {
            // Remote fetch failed after retries - log warning and skip update
            println!(
                "cargo:warning=failed to fetch schema for {}: {}, skipped update",
                toml_path.display(),
                e
            );
            return Ok(());
        }
    };

    // Compare sha256 hashes
    let local_hash = text_file_hash(&local_schema_path)?;

    // If hashes match, no need to update
    if remote_hash == local_hash {
        return Ok(());
    }

    // Hashes differ - replace with relative path
    let relative_path = calculate_relative_path(toml_path, &local_schema_path)?;
    let new_schema_line = format!("#:schema {}", relative_path);

    if schema_line.trim() == new_schema_line.trim() {
        return Ok(());
    }

    rewrite_file_lines(toml_path, |line| {
        if line.trim().starts_with(SCHEMA_PREFIX) {
            new_schema_line.clone()
        } else {
            line.to_string()
        }
    })
}

/// Finds a local schema file by filename in the schema/json directory.
fn find_local_schema_file(filename: &str) -> Result<PathBuf> {
    let schema_dir = Path::new(JSON_SCHEMA_DIR);
    let schema_path = schema_dir.join(filename);

    if schema_path.exists() {
        Ok(schema_path)
    } else {
        Err(anyhow!(
            "Local schema file not found: {}",
            schema_path.display()
        ))
    }
}

/// Middleware that adds retry logic with exponential backoff delay and timeout.
fn with_retry<F>(fetch: F, max_attempts: u32) -> impl Fn(&str) -> Result<Hash>
where
    F: Fn(&str, u32) -> Result<Hash>,
{
    move |url: &str| {
        let mut last_error = None;
        for attempt in 1..=max_attempts {
            match fetch(url, attempt) {
                Ok(hash) => return Ok(hash),
                Err(e) => {
                    if attempt < max_attempts {
                        let delay_ms = 125 * (1 << (attempt - 1));
                        println!(
                            "cargo:warning=retrying {}, attempt {}/{}, delay {}ms",
                            url, attempt, max_attempts, delay_ms
                        );
                        std::thread::sleep(Duration::from_millis(delay_ms));
                    }
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap())
    }
}

/// Middleware that caches results (both successful and failed) by URL.
fn with_cache<F>(fetch: F) -> impl Fn(&str) -> Result<Hash>
where
    F: Fn(&str) -> Result<Hash>,
{
    let cache = std::sync::Mutex::new(HashMap::<String, Result<Hash, String>>::new());
    move |url: &str| {
        let mut cache = cache.lock().unwrap();
        if let Some(cached) = cache.get(url) {
            return cached.clone().map_err(|e| anyhow!("{}", e));
        }
        let result = fetch(url);
        cache.insert(
            url.to_string(),
            result.as_ref().copied().map_err(|e| e.to_string()),
        );
        result
    }
}

/// Fetches content from a URL and returns its SHA256 hash.
/// Uses exponential timeout (2s, 4s, 8s) based on the attempt number.
fn fetch_and_hash_url(url: &str, attempt: u32) -> Result<Hash> {
    let timeout = BASE_FETCH_TIMEOUT * (1 << (attempt - 1));

    eprintln!(
        "termframe: fetching {} (timeout: {}s)",
        url,
        timeout.as_secs()
    );

    let start = Instant::now();
    let agent = {
        ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .tls_config(tls_config())
            .build()
            .new_agent()
    };

    let result = agent.get(url).call();
    let elapsed = start.elapsed();

    match result {
        Ok(mut response) => {
            eprintln!(
                "termframe: fetched {} in {:.2}s",
                url,
                elapsed.as_secs_f64()
            );
            text_reader_hash(std::io::BufReader::new(response.body_mut().as_reader()))
        }
        Err(e) => {
            eprintln!(
                "termframe: failed to fetch {} in {:.2}s: {}",
                url,
                elapsed.as_secs_f64(),
                e
            );
            Err(anyhow!("{}", e))
        }
    }
}

// On Windows, use native-tls to avoid ring/clang requirement
#[cfg(target_os = "windows")]
fn tls_config() -> tls::TlsConfig {
    tls::TlsConfig::builder()
        .provider(tls::TlsProvider::NativeTls)
        .root_certs(tls::RootCerts::PlatformVerifier)
        .build()
}

#[cfg(not(target_os = "windows"))]
fn tls_config() -> tls::TlsConfig {
    tls::TlsConfig::builder().build()
}

/// Computes the SHA256 hash of a text file.
fn text_file_hash(path: &Path) -> Result<Hash> {
    let file = File::open(path).map_err(|e| anyhow!("Failed to open {}: {}", path.display(), e))?;
    text_reader_hash(file)
}

/// Computes the SHA256 hash of text content from a reader.
/// Processes content line by line to ensure consistent hashing across platforms.
fn text_reader_hash<R: std::io::Read>(reader: R) -> Result<Hash> {
    let mut hasher = Sha256::new();
    for line in std::io::BufReader::new(reader).lines() {
        let line = line.map_err(|e| anyhow!("Failed to read line for hashing: {}", e))?;
        hasher.update(line);
        hasher.update(b"\n");
    }
    Ok(hasher.finalize().into())
}

/// Rewrites a file by applying a transformation function to each line.
/// Preserves the final newline character if the original file had one.
fn rewrite_file_lines<F>(path: &Path, transform: F) -> Result<()>
where
    F: Fn(&str) -> String,
{
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read file {}: {}", path.display(), e))?;

    let new_content: String = content
        .lines()
        .map(transform)
        .collect::<Vec<_>>()
        .join("\n");

    let new_content = if content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    fs::write(path, new_content)
        .map_err(|e| anyhow!("Failed to write file {}: {}", path.display(), e))?;

    Ok(())
}

/// Calculates the relative path from one file to another.
/// Used to generate relative schema paths in TOML files.
fn calculate_relative_path(from: &Path, to: &Path) -> Result<String> {
    let from_dir = from
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent directory of {}", from.display()))?;

    let from_components: Vec<_> = from_dir.components().collect();
    let to_components: Vec<_> = to.components().collect();

    let common_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let up_levels = from_components.len() - common_len;
    let mut rel_path = String::new();

    for _ in 0..up_levels {
        rel_path.push_str("../");
    }

    for component in &to_components[common_len..] {
        if let std::path::Component::Normal(comp) = component {
            if !rel_path.is_empty() && !rel_path.ends_with('/') {
                rel_path.push('/');
            }
            rel_path.push_str(
                comp.to_str()
                    .ok_or_else(|| anyhow!("Invalid path component"))?,
            );
        }
    }

    Ok(rel_path)
}

type Hash = [u8; 32];
