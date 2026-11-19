# Rust FTP Server — starter repository

This repository contains the Rust project scaffold, Docker files, sample JSON config,
and a GitHub Actions release matrix for Windows, Linux and macOS.

## Important implementation status

**This is a build/release scaffold, not a finished FTP server.** The current `main.rs`
validates the config and intentionally exits without opening an FTP listener. It does
not yet authenticate users or transfer files.

That fail-closed behavior is intentional: the requested configuration maps multiple
independent host directories to FTP logins. A correct implementation needs a
per-user identity/home-directory provider and rooted filesystem backend. Simply using
one shared filesystem backend would risk exposing one user's files to another user.
The scaffold must not be presented as a working FTP service until that adapter is
implemented and tested.

The dependency selected for the eventual FTP protocol layer is
[`libunftp`](https://docs.rs/libunftp/), a Rust FTP/FTPS server library.

## Config

`config.json` is loaded from the same directory as the executable.

```json
{
  "host": "0.0.0.0",
  "port": 2121,
  "paths": [
    {
      "path": "D:/",
      "user": "pc",
      "password": "CHANGE_ME"
    },
    {
      "path": "./MOV",
      "user": "media",
      "password": "CHANGE_ME_TOO"
    }
  ],
  "passive_port_start": 50000,
  "passive_port_end": 50100
}
```

The extra passive port range is needed for FTP data connections and Docker port mapping.
Use distinct usernames for distinct roots. The sample credentials are placeholders;
do not expose FTP with weak passwords.

## Build locally

Install Rust 1.85+ and run:

```bash
cargo build --release
```

The executable is written to `target/release/` (or `target/release/` with `.exe` on Windows).

## Docker

```bash
docker compose build
docker compose up -d
```

Update the volume mappings and paths in `config.json` for the host OS first.
The sample compose file maps `./MOV` to `/data/MOV`; use `/data/MOV` in the
container config, not a Windows drive path.

## GitHub Releases

Push the project to a GitHub repository, then create and push a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds:
- Windows x86_64
- Linux x86_64
- macOS Apple Silicon (aarch64)
- macOS Intel (x86_64)

It packages each executable together with `config.json`. A GitHub Actions run is
required to produce those platform-specific binaries; this workspace cannot create
native release artifacts for all three operating systems.

## Security notes

- Plain FTP sends credentials and data without encryption. Use a trusted private LAN
  or implement FTPS before exposing it to the public internet.
- Restrict the passive port range in the firewall and publish the same range in Docker.
- Use strong, unique passwords.
- Run the server as an unprivileged OS user.
- Never mount a broad host filesystem root into a container unless that access is intended.

---

```sh
git add .
git commit -m "Update release workflow"
git push

git tag v0.1.1
git push origin v0.1.1
```
