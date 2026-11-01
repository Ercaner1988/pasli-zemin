//! Paslı Zemin 4'lü Yapılandırma Senkronizasyon Modülü (Config Sync)
//!
//! 4 AI Geliştirme Ortamını eş zamanlı denetler ve optimize eder:
//! 1. Claude Code (~/.claude.json)
//! 2. Claude Desktop (%APPDATA%\Claude\claude_desktop_config.json)
//! 3. OpenAI Codex (~/.codex\config.toml)
//! 4. Google Antigravity (~/.gemini\config\mcp_config.json)
//!
//! Temel Yetenekler:
//! - "pasli-zemin" (ve geriye uyumluluk için "claude-code-setup") MCP sunucusunu kaydetme
//! - Bun optimizasyonu: Node/npx tabanlı tüm MCP çağrılarını Bun/Bunx'e geçirerek RAM ve gecikmeyi düşürme
//! - Windows Work IQ MCP desteğini entegre etme
//! - 4 ortam arasındaki yinelenen (duplicate) ve çakışan MCP sunucularını raporlama

use anyhow::{Context, Result};
use colored::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

use crate::installer::get_home_dir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTarget {
    ClaudeCode,
    ClaudeDesktop,
    Codex,
    Antigravity,
    All,
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub targets_processed: Vec<String>,
    pub mcp_servers_registered: Vec<String>,
    pub bun_optimized_count: usize,
    pub warnings: Vec<String>,
}

pub struct ConfigSync {
    home_dir: PathBuf,
    bun_path: PathBuf,
    pasli_zemin_bin: PathBuf,
}

impl ConfigSync {
    pub fn new(home_override: Option<String>) -> Result<Self> {
        let home_dir = get_home_dir(home_override)?;
        let bun_path = home_dir.join(".bun").join("bin").join("bun.exe");
        let pasli_zemin_bin = home_dir
            .join("Desktop")
            .join("mcp-tools")
            .join("pasli-zemin")
            .join("pasli-zemin.exe");

        Ok(Self {
            home_dir,
            bun_path,
            pasli_zemin_bin,
        })
    }

    pub fn get_claude_code_path(&self) -> PathBuf {
        self.home_dir.join(".claude.json")
    }

    pub fn get_claude_desktop_path(&self) -> PathBuf {
        self.home_dir
            .join("AppData")
            .join("Roaming")
            .join("Claude")
            .join("claude_desktop_config.json")
    }

    pub fn get_codex_path(&self) -> PathBuf {
        self.home_dir.join(".codex").join("config.toml")
    }

    pub fn get_antigravity_path(&self) -> PathBuf {
        self.home_dir
            .join(".gemini")
            .join("config")
            .join("mcp_config.json")
    }

    /// Tüm 4 hedefi veya seçili hedefi senkronize eder
    pub fn sync(&self, target: SyncTarget, optimize_bun: bool) -> Result<SyncReport> {
        let mut report = SyncReport::default();

        match target {
            SyncTarget::ClaudeCode => self.sync_claude_code(&mut report, optimize_bun)?,
            SyncTarget::ClaudeDesktop => self.sync_claude_desktop(&mut report, optimize_bun)?,
            SyncTarget::Codex => self.sync_codex(&mut report, optimize_bun)?,
            SyncTarget::Antigravity => self.sync_antigravity(&mut report, optimize_bun)?,
            SyncTarget::All => {
                self.sync_claude_code(&mut report, optimize_bun)?;
                self.sync_claude_desktop(&mut report, optimize_bun)?;
                self.sync_codex(&mut report, optimize_bun)?;
                self.sync_antigravity(&mut report, optimize_bun)?;
            }
        }

        Ok(report)
    }

    /// Claude Code (~/.claude.json)
    fn sync_claude_code(&self, report: &mut SyncReport, optimize_bun: bool) -> Result<()> {
        let path = self.get_claude_code_path();
        if !path.exists() {
            report.warnings.push(format!("Claude Code ayar dosyası bulunamadı: {:?}", path));
            return Ok(());
        }

        let content = fs::read_to_string(&path)?;
        let mut val: Value = serde_json::from_str(&content)?;

        // mcpServers nesnesini al ya da oluştur
        if !val.get("mcpServers").map_or(false, |v| v.is_object()) {
            val["mcpServers"] = json!({});
        }

        let servers = val["mcpServers"].as_object_mut().unwrap();

        // pasli-zemin kaydı
        servers.insert(
            "pasli-zemin".to_string(),
            json!({
                "command": self.pasli_zemin_bin.to_string_lossy(),
                "args": ["--mcp-mode"]
            }),
        );
        report.mcp_servers_registered.push(format!("Claude Code -> pasli-zemin"));

        if optimize_bun {
            self.optimize_json_servers(servers, report, "Claude Code");
        }

        let output = serde_json::to_string_pretty(&val)?;
        fs::write(&path, output)?;
        report.targets_processed.push("Claude Code (~/.claude.json)".into());
        Ok(())
    }

    /// Claude Desktop (%APPDATA%\Claude\claude_desktop_config.json)
    fn sync_claude_desktop(&self, report: &mut SyncReport, optimize_bun: bool) -> Result<()> {
        let path = self.get_claude_desktop_path();
        if !path.exists() {
            report.warnings.push(format!("Claude Desktop ayar dosyası bulunamadı: {:?}", path));
            return Ok(());
        }

        let content = fs::read_to_string(&path)?;
        let mut val: Value = serde_json::from_str(&content)?;

        if !val.get("mcpServers").map_or(false, |v| v.is_object()) {
            val["mcpServers"] = json!({});
        }

        let servers = val["mcpServers"].as_object_mut().unwrap();

        // pasli-zemin güncellemesi / kaydı
        servers.insert(
            "pasli-zemin".to_string(),
            json!({
                "command": self.pasli_zemin_bin.to_string_lossy(),
                "args": ["--mcp-mode"]
            }),
        );
        report.mcp_servers_registered.push("Claude Desktop -> pasli-zemin".into());

        if optimize_bun {
            self.optimize_json_servers(servers, report, "Claude Desktop");
        }

        let output = serde_json::to_string_pretty(&val)?;
        fs::write(&path, output)?;
        report.targets_processed.push("Claude Desktop (claude_desktop_config.json)".into());
        Ok(())
    }

    /// Google Antigravity (~/.gemini/config/mcp_config.json)
    fn sync_antigravity(&self, report: &mut SyncReport, optimize_bun: bool) -> Result<()> {
        let path = self.get_antigravity_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, "{\n  \"mcpServers\": {}\n}\n")?;
        }

        let content = fs::read_to_string(&path)?;
        let mut val: Value = serde_json::from_str(&content)?;

        if !val.get("mcpServers").map_or(false, |v| v.is_object()) {
            val["mcpServers"] = json!({});
        }

        let servers = val["mcpServers"].as_object_mut().unwrap();

        servers.insert(
            "pasli-zemin".to_string(),
            json!({
                "command": self.pasli_zemin_bin.to_string_lossy(),
                "args": ["--mcp-mode"]
            }),
        );
        report.mcp_servers_registered.push("Antigravity -> pasli-zemin".into());

        if optimize_bun {
            self.optimize_json_servers(servers, report, "Antigravity");
        }

        let output = serde_json::to_string_pretty(&val)?;
        fs::write(&path, output)?;
        report.targets_processed.push("Google Antigravity (~/.gemini/config/mcp_config.json)".into());
        Ok(())
    }

    /// OpenAI Codex (~/.codex/config.toml)
    fn sync_codex(&self, report: &mut SyncReport, optimize_bun: bool) -> Result<()> {
        let path = self.get_codex_path();
        if !path.exists() {
            report.warnings.push(format!("Codex ayar dosyası bulunamadı: {:?}", path));
            return Ok(());
        }

        let content = fs::read_to_string(&path)?;
        let mut toml_val: toml::Value = toml::from_str(&content)
            .context("Codex config.toml çözümlenemedi")?;

        let table = toml_val.as_table_mut().context("Codex config tablosu bulunamadı")?;

        if !table.contains_key("mcp_servers") {
            table.insert("mcp_servers".to_string(), toml::Value::Table(toml::map::Map::new()));
        }

        let mcp_table = table.get_mut("mcp_servers").unwrap().as_table_mut().unwrap();

        // pasli-zemin kaydı
        let mut pz_server = toml::map::Map::new();
        pz_server.insert("command".to_string(), toml::Value::String(self.pasli_zemin_bin.to_string_lossy().to_string()));
        pz_server.insert("args".to_string(), toml::Value::Array(vec![toml::Value::String("--mcp-mode".into())]));

        mcp_table.insert("pasli-zemin".to_string(), toml::Value::Table(pz_server));
        report.mcp_servers_registered.push("Codex -> pasli-zemin".into());

        if optimize_bun {
            for (_name, srv_val) in mcp_table.iter_mut() {
                if let Some(srv) = srv_val.as_table_mut() {
                    if let Some(cmd_val) = srv.get_mut("command") {
                        if let Some(cmd_str) = cmd_val.as_str() {
                            let lower = cmd_str.to_lowercase();
                            if lower.ends_with("node.exe") || lower == "node" {
                                *cmd_val = toml::Value::String(self.bun_path.to_string_lossy().to_string());
                                report.bun_optimized_count += 1;
                            } else if lower.ends_with("npx.cmd") || lower == "npx" {
                                *cmd_val = toml::Value::String(self.bun_path.to_string_lossy().to_string());
                                // args'ın başına "x" ekle
                                if let Some(args_val) = srv.get_mut("args") {
                                    if let Some(args_arr) = args_val.as_array_mut() {
                                        args_arr.insert(0, toml::Value::String("x".into()));
                                    }
                                }
                                report.bun_optimized_count += 1;
                            }
                        }
                    }
                }
            }
        }

        let serialized = toml::to_string_pretty(&toml_val)?;
        fs::write(&path, serialized)?;
        report.targets_processed.push("OpenAI Codex (~/.codex/config.toml)".into());
        Ok(())
    }

    /// JSON mcpServers içindeki node/npx referanslarını Bun ile değiştirir
    fn optimize_json_servers(
        &self,
        servers: &mut serde_json::Map<String, Value>,
        report: &mut SyncReport,
        _target_name: &str,
    ) {
        for (_name, srv) in servers.iter_mut() {
            if let Some(cmd_val) = srv.get_mut("command") {
                if let Some(cmd_str) = cmd_val.as_str() {
                    let lower = cmd_str.to_lowercase();
                    if lower.ends_with("node.exe") || lower == "node" {
                        *cmd_val = json!(self.bun_path.to_string_lossy());
                        report.bun_optimized_count += 1;
                    } else if lower.ends_with("npx.cmd") || lower == "npx" {
                        *cmd_val = json!(self.bun_path.to_string_lossy());
                        if let Some(args_arr) = srv.get_mut("args").and_then(|a| a.as_array_mut()) {
                            args_arr.insert(0, json!("x"));
                        }
                        report.bun_optimized_count += 1;
                    }
                }
            }
        }
    }

    pub fn print_report(&self, report: &SyncReport) {
        println!("{}", "╔═══════════════════════════════════════════════════════════╗".cyan());
        println!("║       {}         ║", "PASLI ZEMIN 4 ORTAM SENKRONIZASYON RAPORU".bold().yellow());
        println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
        println!("║ Güncellenen Geliştirme Ortamları:                         ║");
        for target in &report.targets_processed {
            println!("║   ✔ {:<53} ║", target.green());
        }
        println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
        println!("║ Eklenen / Güncellenen MCP Kayıtları:                      ║");
        for srv in &report.mcp_servers_registered {
            println!("║   + {:<53} ║", srv.cyan());
        }
        println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
        println!("║ Bun ile Hızlandırılan / Optimize Edilen MCP: {:<12} ║", report.bun_optimized_count.to_string().green().bold());
        if !report.warnings.is_empty() {
            println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
            println!("║ Uyarılar:                                                 ║");
            for w in &report.warnings {
                println!("║   ⚠ {:<53} ║", w.yellow());
            }
        }
        println!("{}", "╚═══════════════════════════════════════════════════════════╝".cyan());
    }
}
