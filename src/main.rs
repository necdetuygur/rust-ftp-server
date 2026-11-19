use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{env, fs, net::SocketAddr, path::PathBuf};

#[derive(Debug, Deserialize)]
struct Config {
    host: String,
    port: u16,
    paths: Vec<PathEntry>,
    #[serde(default = "default_pasv_start")]
    passive_port_start: u16,
    #[serde(default = "default_pasv_end")]
    passive_port_end: u16,
}

#[derive(Debug, Deserialize)]
struct PathEntry {
    path: String,
    user: String,
    password: String,
}

fn default_pasv_start() -> u16 { 50000 }
fn default_pasv_end() -> u16 { 50100 }

fn config_path() -> Result<PathBuf> {
    let exe = env::current_exe().context("Cannot locate executable")?;
    let dir = exe.parent().context("Executable has no parent directory")?;
    Ok(dir.join("config.json"))
}

fn main() -> Result<()> {
    let cfg_path = config_path()?;
    let raw = fs::read_to_string(&cfg_path)
        .with_context(|| format!("Cannot read {}", cfg_path.display()))?;
    let cfg: Config = serde_json::from_str(&raw).context("Invalid config.json")?;

    if cfg.paths.is_empty() {
        bail!("config.json must contain at least one entry in 'paths'");
    }
    if cfg.passive_port_start > cfg.passive_port_end {
        bail!("passive_port_start must be <= passive_port_end");
    }
    for entry in &cfg.paths {
        if entry.user.trim().is_empty() || entry.password.is_empty() {
            bail!("Every path entry must have a non-empty user and password");
        }
        let p = PathBuf::from(&entry.path);
        if !p.exists() || !p.is_dir() {
            bail!("Configured directory does not exist or is not a directory: {}", entry.path);
        }
    }

    // This starter intentionally refuses to start with multiple independent roots:
    // libunftp's standard filesystem backend is rooted once per server instance.
    // A per-login root provider is required to safely map each config path to a user.
    // We fail closed rather than accidentally expose a shared parent directory.
    if cfg.paths.len() != 1 {
        bail!(
            "This starter build validates your config but does not yet enable multi-root user mapping. \
             Configure exactly one path entry for now. Multi-root isolation must be implemented \
             with a per-user UserDetailProvider/rooted storage backend before enabling multiple users."
        );
    }

    let entry = &cfg.paths[0];
    let root = fs::canonicalize(&entry.path)
        .with_context(|| format!("Cannot resolve directory {}", entry.path))?;
    let username = entry.user.clone();
    let password = entry.password.clone();

    // Prevent accidentally starting with an insecure example password.
    if password == "000000" {
        eprintln!("WARNING: config uses the example password '000000'. Change it before exposing the server.");
    }

    // The stock AnonymousAuthenticator is deliberately not used. This single-user
    // starter keeps authentication closed until the per-user auth adapter is wired in.
    // See README for the exact next implementation step and security notes.
    let _ = (username, password, root);

    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .context("Invalid host/port combination")?;
    println!("Configuration validated for FTP bind address {addr}");
    println!("Configured root: {}", entry.path);
    println!(
        "Passive data ports configured: {}-{}",
        cfg.passive_port_start, cfg.passive_port_end
    );
    println!("This scaffold does not start an FTP listener yet; see README.");
    Ok(())
}
