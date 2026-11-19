//! A small, config-driven FTP server with one isolated home directory per login.
//!
//! Authentication and per-user roots come from `config.json`. libunftp's filesystem
//! backend is opened once at a common ancestor of all configured directories and is
//! then narrowed to the logged-in user's home through `UserDetail::home()`. That is
//! the library-supported way to keep users inside their own directory.

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use libunftp::ServerBuilder;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fmt, fs,
    net::SocketAddr,
    ops::RangeInclusive,
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use unftp_core::auth::{
    AuthenticationError, Authenticator, Credentials, Principal, UserDetail, UserDetailError,
    UserDetailProvider,
};
use unftp_sbe_fs::Filesystem;

const DEFAULT_CONFIG_FILE: &str = "config.json";
const GREETING: &str = "Welcome to the config-driven Rust FTP server";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Config {
    host: String,
    port: u16,
    paths: Vec<PathEntry>,
    #[serde(default = "default_pasv_start")]
    passive_port_start: u16,
    #[serde(default = "default_pasv_end")]
    passive_port_end: u16,
    /// Address advertised in PASV replies. Useful behind Docker/NAT.
    #[serde(default)]
    passive_host: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PathEntry {
    path: String,
    user: String,
    password: String,
}

#[derive(Debug)]
struct FtpUser {
    password: String,
    home: PathBuf,
}

#[derive(Debug)]
struct RuntimeConfig {
    bind: SocketAddr,
    passive_ports: RangeInclusive<u16>,
    passive_host: Option<String>,
    storage_root: PathBuf,
    users: Arc<HashMap<String, FtpUser>>,
}

fn default_pasv_start() -> u16 {
    50000
}

fn default_pasv_end() -> u16 {
    50100
}

fn load_config(path: &Path) -> Result<RuntimeConfig> {
    let raw = fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
    let cfg: Config = serde_json::from_str(&raw).context("Invalid config.json")?;

    if cfg.paths.is_empty() {
        bail!("config.json must contain at least one entry in 'paths'");
    }
    if cfg.passive_port_start > cfg.passive_port_end {
        bail!("passive_port_start must be <= passive_port_end");
    }
    if cfg.passive_port_start < 1024 {
        bail!("passive_port_start must be >= 1024 (privileged ports are not used)");
    }

    let mut users = HashMap::new();
    for entry in &cfg.paths {
        let username = entry.user.trim();
        if username.is_empty() {
            bail!("Every path entry needs a non-empty 'user'");
        }
        if entry.password.is_empty() {
            bail!("User '{username}' has an empty password");
        }
        if users.contains_key(username) {
            bail!("Duplicate username '{username}' in config.json");
        }

        let home = fs::canonicalize(&entry.path)
            .with_context(|| format!("Cannot resolve directory '{}' for user '{username}'", entry.path))?;
        if !home.is_dir() {
            bail!("Configured path '{}' for user '{username}' is not a directory", entry.path);
        }
        if home.parent().is_none() {
            bail!("Refusing to expose the filesystem root as the home of user '{username}'");
        }

        users.insert(
            username.to_owned(),
            FtpUser {
                password: entry.password.clone(),
                home,
            },
        );
    }

    let homes: Vec<PathBuf> = users.values().map(|u| u.home.clone()).collect();
    let storage_root = common_storage_root(&homes)?;

    let bind: SocketAddr = format!("{}:{}", cfg.host, cfg.port)
        .parse()
        .context("Invalid host/port combination")?;

    Ok(RuntimeConfig {
        bind,
        passive_ports: cfg.passive_port_start..=cfg.passive_port_end,
        passive_host: cfg.passive_host.filter(|h| !h.trim().is_empty()),
        storage_root,
        users: Arc::new(users),
    })
}

/// Deepest directory that contains every configured home. We deliberately step one
/// level higher when the common directory is itself a home: `Filesystem::enter()`
/// needs a non-empty relative path to narrow the session to that user's directory.
fn common_storage_root(homes: &[PathBuf]) -> Result<PathBuf> {
    let first: Vec<Component<'_>> = homes[0].components().collect();
    let mut root = PathBuf::new();

    for (index, component) in first.iter().enumerate() {
        if homes
            .iter()
            .all(|home| home.components().nth(index) == Some(*component))
        {
            root.push(component.as_os_str());
        } else {
            break;
        }
    }

    if homes.iter().any(|home| home == &root) && !root.pop() {
        bail!("Refusing to use '/' as the FTP storage root");
    }

    for home in homes {
        if home == &root || !home.starts_with(&root) {
            bail!("Home {} is not inside storage root {}", home.display(), root.display());
        }
    }

    Ok(root)
}

// ---------------------------------------------------------------------------
// Authentication and per-user identity
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ConfigAuthenticator {
    users: Arc<HashMap<String, FtpUser>>,
}

#[async_trait]
impl Authenticator for ConfigAuthenticator {
    async fn authenticate(&self, username: &str, creds: &Credentials) -> Result<Principal, AuthenticationError> {
        let Some(user) = self.users.get(username) else {
            return Err(AuthenticationError::BadUser);
        };

        match creds.password.as_deref() {
            Some(password) if password == user.password => Ok(Principal {
                username: username.to_owned(),
            }),
            _ => Err(AuthenticationError::BadPassword),
        }
    }

    fn name(&self) -> &str {
        "config-file"
    }
}

#[derive(Debug)]
struct SessionUser {
    username: String,
    home: PathBuf,
}

impl fmt::Display for SessionUser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.username)
    }
}

impl UserDetail for SessionUser {
    fn home(&self) -> Option<&Path> {
        Some(&self.home)
    }
}

#[derive(Debug)]
struct ConfigUserProvider {
    users: Arc<HashMap<String, FtpUser>>,
}

#[async_trait]
impl UserDetailProvider for ConfigUserProvider {
    type User = SessionUser;

    async fn provide_user_detail(&self, principal: &Principal) -> Result<Self::User, UserDetailError> {
        let user = self
            .users
            .get(&principal.username)
            .ok_or_else(|| UserDetailError::UserNotFound {
                username: principal.username.clone(),
            })?;

        Ok(SessionUser {
            username: principal.username.clone(),
            home: user.home.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// CLI / config discovery
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Cli {
    config: Option<PathBuf>,
    check_only: bool,
}

fn usage() -> &'static str {
    "Usage: rust-ftp-server [--config PATH] [--check]\n\n  -c, --config PATH  Load a different config.json\n      --check         Validate config and exit without starting the listener\n  -h, --help         Show this help"
}

fn parse_args() -> Result<Cli> {
    let mut cli = Cli::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            "-c" | "--config" => {
                let value = args.next().context("Missing value for --config")?;
                cli.config = Some(PathBuf::from(value));
            }
            "--check" => cli.check_only = true,
            _ if arg.starts_with("--config=") => {
                cli.config = Some(PathBuf::from(arg.trim_start_matches("--config=")));
            }
            _ => bail!("Unknown argument '{arg}'\n{}", usage()),
        }
    }

    Ok(cli)
}

fn resolve_config_path(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    if let Ok(path) = env::var("FTP_CONFIG") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let local = cwd.join(DEFAULT_CONFIG_FILE);
    if local.is_file() {
        return local;
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_exe = dir.join(DEFAULT_CONFIG_FILE);
            if next_to_exe.is_file() {
                return next_to_exe;
            }
        }
    }
    local
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::try_init();
    let cli = parse_args()?;
    let config_path = resolve_config_path(cli.config);
    let config = load_config(&config_path)?;

    let RuntimeConfig {
        bind,
        passive_ports,
        passive_host,
        storage_root,
        users,
    } = config;

    println!("Config:       {}", config_path.display());
    println!("Listening on: ftp://{bind}");
    println!("Storage root: {}", storage_root.display());
    println!("Passive ports: {}-{}", passive_ports.start(), passive_ports.end());
    if let Some(host) = &passive_host {
        println!("PASV host:    {host}");
    }
    for (name, user) in users.iter() {
        println!("  {name:<12} -> {}", user.home.display());
    }

    if cli.check_only {
        println!("Configuration OK (check mode; listener not started)");
        return Ok(());
    }

    let root_for_backend = storage_root.clone();
    let authenticator: Arc<dyn Authenticator + Send + Sync> = Arc::new(ConfigAuthenticator {
        users: Arc::clone(&users),
    });
    let provider = ConfigUserProvider { users };

    let mut builder = ServerBuilder::<Filesystem, SessionUser>::with_user_detail_provider(
        Box::new(move || {
            Filesystem::new(root_for_backend.clone()).expect("configured storage root must stay accessible")
        }),
        Arc::new(provider),
    )
    .authenticator(authenticator)
    .greeting(GREETING)
    .passive_ports(passive_ports);

    if let Some(host) = passive_host.as_deref() {
        builder = builder.passive_host(host);
    }

    let server = builder.build().context("Cannot build FTP server")?;
    server
        .listen(bind.to_string())
        .await
        .context("FTP server stopped with an error")?;

    Ok(())
}
