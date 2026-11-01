**🌍 [Türkçe](INSTALLATION.md) | [English](INSTALLATION.en.md) | [العربية](INSTALLATION.ar.md) | [日本語](INSTALLATION.ja.md) | [中文](INSTALLATION.zh.md) | [Русский](INSTALLATION.ru.md) | [Español](INSTALLATION.es.md)**

# Claude Code Standalone Installation Guide (Rust-Core Single Binary)

This guide provides step-by-step instructions for installing and configuring **Paslı Zemin** (`pasli-zemin`), a Rust-core CLI management, security, and memory tool across different platforms.

> **Honesty note:** The binary you run is pure Rust and self-contained. The **installers in this guide are not Rust**: `install-windows.ps1` is a PowerShell script and `install-macos.sh` is a Bash script. Release packages are produced by `package-extension.py` (Python). GitHub language statistics: **90.5% Rust, 3.5% Shell, 3.2% Python, 2.8% PowerShell** (measured line ratio: 91.2% Rust, 8.8% PowerShell + Bash + Python).

---

## 🎯 1. Overview

- **Single Binary Runtime:** Running the tool requires no Rust, Python, or Node installation. The installers themselves are PowerShell/Bash scripts, and the `.mcpb` packager is Python — those run around the tool, not inside it.
- **Cross-Platform:** Native performance on Windows (x64), Linux (x64), and macOS (x64 / ARM64).
- **Offline After First Run:** Semantic search downloads the embedding model and the ONNX Runtime binary from Hugging Face on first use, then works fully offline from the `$HOME/.claude/fastembed_cache` directory.

---

## 📥 2. Method 1: Pre-Compiled Binary Download (Recommended)

Download the pre-compiled binary for your operating system directly from GitHub Releases.

### Windows (x64)
Download and execute via PowerShell:
```powershell
# Download binary release asset
Invoke-WebRequest -Uri "https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-windows-x86_64.exe" -OutFile "pasli-zemin.exe"

# Execute setup & memory initialization
.\pasli-zemin.exe install --hooks
```

### Linux (x64)
Download via terminal and grant execution permission:
```bash
# Download binary release asset
curl -LO https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-linux-x86_64

# Grant execution permission
chmod +x pasli-zemin-linux-x86_64

# Execute setup
./pasli-zemin-linux-x86_64 install --hooks
```

### macOS (x64)
```bash
# Download binary release asset
curl -LO https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-macos-x86_64

# Grant execution permission
chmod +x pasli-zemin-macos-x86_64

# Execute setup
./pasli-zemin-macos-x86_64 install --hooks
```

---

## 🛠️ 3. Method 2: Building from Source (Cargo)

If you have a Rust toolchain (`cargo` 1.80+) installed:

```bash
# Clone repository
git clone https://github.com/Ercaner1988/pasli-zemin.git
cd pasli-zemin

# Build release binary
cargo build --release

# Execute setup
./target/release/pasli-zemin install --hooks
```

To install system-wide via cargo:
```bash
cargo install --path .
pasli-zemin install --hooks
```

---

## ⚙️ 4. Post-Installation Verification & Diagnostics

Verify system diagnostics after setup:

```bash
# Run environment diagnostics
pasli-zemin status

# Run diagnostic verification test suite
pasli-zemin test
```

---

## 🛡️ 5. Security Auditing & Git Hook Setup

Audit configuration security and install pre-commit branch protection hooks:

```bash
# Security audit with auto-fix
pasli-zemin security-audit --fix

# Install pre-commit hook into a repository
pasli-zemin install-hooks --repo-dir .
```

---

## 📚 6. Related Documentation

- [Full Documentation (README.md)](README.md)
- [Deployment Guide (DEPLOYMENT_GUIDE.md)](DEPLOYMENT_GUIDE.md)
- [Troubleshooting Guide (TROUBLESHOOTING.md)](docs/TROUBLESHOOTING.md)
