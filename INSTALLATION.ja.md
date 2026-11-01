**🌍 [Türkçe](INSTALLATION.md) | [English](INSTALLATION.en.md) | [العربية](INSTALLATION.ar.md) | [日本語](INSTALLATION.ja.md) | [中文](INSTALLATION.zh.md) | [Русский](INSTALLATION.ru.md) | [Español](INSTALLATION.es.md)**

# Claude Code スタンドアロンインストールガイド (Rust コア単一バイナリ)

このガイドでは、**Claude Code** 環境を管理する Rust コアのローカル CLI ツール (`pasli-zemin`) を各プラットフォームにインストールおよび設定する方法を解説します。

> **誠実性に関する注記:** 実行されるバイナリは純粋な Rust であり、自己完結しています。しかし本ガイドの**インストーラーは Rust ではありません**。`install-windows.ps1` は PowerShell スクリプト、`install-macos.sh` は Bash スクリプトです。リリースパッケージは `package-extension.py` (Python) で生成されます。GitHub の言語統計: **Rust 90.5%, Shell 3.5%, Python 3.2%, PowerShell 2.8%** (計測された行数構成比: Rust 91.2%、PowerShell + Bash + Python 8.8%)。

---

## 🎯 1. 概要

- **単一バイナリ (Single Binary):** Shell (`.sh`) および Python (`.py`) スクリプトへの依存関係を完全に排除。
- **クロスプラットフォーム:** Windows (x64)、Linux (x64)、macOS (x64 / ARM64) でネイティブ動作。
- **外部依存なし:** 事前ビルド済みバイナリのダウンロードまたは `cargo` によるビルドで数秒でインストール可能。

---

## 📥 2. 方法 1: ビルド済みバイナリのダウンロード (推奨)

GitHub Release ページからお使いの OS に適したバイナリを直接ダウンロードできます。

### Windows (x64)
PowerShell 経由で実行可能バイナリをダウンロード:
```powershell
# バイナリアセットのダウンロード
Invoke-WebRequest -Uri "https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-windows-x86_64.exe" -OutFile "pasli-zemin.exe"

# 自動セットアップの実行
.\pasli-zemin.exe install --hooks
```

### Linux (x64)
```bash
# バイナリアセットのダウンロード
curl -LO https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-linux-x86_64

# 実行権限の付与
chmod +x pasli-zemin-linux-x86_64

# セットアップの実行
./pasli-zemin-linux-x86_64 install --hooks
```

### macOS (x64)
```bash
# バイナリアセットのダウンロード
curl -LO https://github.com/Ercaner1988/pasli-zemin/releases/latest/download/pasli-zemin-macos-x86_64

# 実行権限の付与
chmod +x pasli-zemin-macos-x86_64

# セットアップの実行
./pasli-zemin-macos-x86_64 install --hooks
```

---

## 🛠️ 3. 方法 2: ソースコードからのビルド (Cargo)

Rust 環境 (`cargo` 1.80 以上) がインストールされている場合:

```bash
# リポジトリのクローン
git clone https://github.com/Ercaner1988/pasli-zemin.git
cd pasli-zemin

# リリリースバイナリのビルド
cargo build --release

# セットアップの実行
./target/release/pasli-zemin install --hooks
```

システム全体にインストールする場合:
```bash
cargo install --path .
pasli-zemin install --hooks
```

---

## ⚙️ 4. インストール後の検証と診断

```bash
# 環境ステータスの診断
pasli-zemin status

# 診断テストスイートの実行
pasli-zemin test
```

---

## 🛡️ 5. セキュリティ監査と Git フックのセットアップ

```bash
# 自動修復付きセキュリティ監査
pasli-zemin security-audit --fix

# リポジトリへの pre-commit フックのインストール
pasli-zemin install-hooks --repo-dir .
```

---

## 📚 6. 関連ドキュメント

- [完全ドキュメント (README.md)](README.md)
- [デプロイメントガイド (DEPLOYMENT_GUIDE.md)](DEPLOYMENT_GUIDE.md)
- [トラブルシューティングガイド (TROUBLESHOOTING.md)](docs/TROUBLESHOOTING.md)
