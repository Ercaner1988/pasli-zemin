//! Paslı Zemin Kütüphane Arayüzü
//!
//! 4 AI geliştirme ortamı (Claude Code, Claude Desktop, OpenAI Codex, Antigravity)
//! için orkestrasyon, donanım bekçisi (QoS & RAM trimming), Windows Work IQ ve Bun MCP yönetimi.

pub mod agent;
pub mod branch_manager;
pub mod cli;
pub mod config_sync;
pub mod governor;
pub mod installer;
pub mod mcp;
pub mod mcp_server;
pub mod memory_engine;
pub mod security;
pub mod tester;
pub mod workiq;

// Geriye dönük uyumluluk için alias
pub use crate as claude_code_setup;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, GovernorAction, WorkIqAction};
use config_sync::{ConfigSync, SyncTarget};
use governor::Governor;
use workiq::WorkIqClient;

pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    // MCP mode: stdin/stdout JSON-RPC protocol
    if cli.mcp_mode {
        return mcp_server::run_mcp_mode();
    }

    // subcommand yoksa yardım bas
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        return Ok(());
    };

    match command {
        Commands::Install {
            skip_prereqs,
            hooks,
            home_dir,
        } => {
            installer::run_install(skip_prereqs, hooks, home_dir)?;
        }
        Commands::Test { home_dir } => {
            tester::run_tests(home_dir)?;
        }
        Commands::McpList { target, home_dir } => {
            mcp::list_mcp_servers(target, home_dir)?;
        }
        Commands::McpSet {
            server,
            command,
            arg,
            env,
            target,
            home_dir,
        } => {
            mcp::mcp_set(&server, command, arg, env, target, home_dir)?;
        }
        Commands::McpUnset {
            server,
            env,
            clear_args,
            remove,
            target,
            home_dir,
        } => {
            mcp::mcp_unset(&server, env, clear_args, remove, target, home_dir)?;
        }
        Commands::McpEnable {
            server,
            target,
            home_dir,
        } => {
            mcp::mcp_toggle(&server, false, target, home_dir)?;
        }
        Commands::McpDisable {
            server,
            target,
            home_dir,
        } => {
            mcp::mcp_toggle(&server, true, target, home_dir)?;
        }
        Commands::MemoryIndex {
            home_dir,
            edge_threshold,
            source,
        } => {
            memory_engine::index_memory(home_dir, edge_threshold, source)?;
        }
        Commands::MemorySearch {
            query,
            mode,
            limit,
            min_score,
            home_dir,
        } => {
            memory_engine::search_memory(&query, &mode, home_dir, limit, min_score)?;
        }
        Commands::MemoryRelated { note, home_dir } => {
            memory_engine::get_related_notes(&note, home_dir)?;
        }
        Commands::MemoryNote {
            title,
            body,
            dir,
            home_dir,
        } => {
            memory_engine::add_memory_note(&title, body.as_deref(), dir, home_dir)?;
        }
        Commands::InstallHooks { repo_dir } => {
            security::install_git_hooks(repo_dir)?;
        }
        Commands::SecurityAudit { fix, home_dir } => {
            security::run_security_audit(fix, home_dir)?;
        }
        Commands::AgentWorkflow {
            branch_type,
            description,
            files,
        } => {
            agent::run_agent_workflow(&branch_type, &description, &files)?;
        }
        Commands::Status => {
            tester::run_tests(None)?;
        }
        Commands::Governor { action } => {
            let gov = Governor::new();
            match action {
                GovernorAction::Run { interval } => {
                    gov.run_daemon(interval)?;
                }
                GovernorAction::Trim => {
                    let stats = gov.scan_and_tune(true)?;
                    gov.print_report(&stats);
                }
            }
        }
        Commands::Sync {
            target,
            optimize_bun,
            home_dir,
        } => {
            let syncer = ConfigSync::new(home_dir)?;
            let sync_target = match target.to_lowercase().as_str() {
                "claude-code" => SyncTarget::ClaudeCode,
                "claude-desktop" => SyncTarget::ClaudeDesktop,
                "codex" => SyncTarget::Codex,
                "antigravity" => SyncTarget::Antigravity,
                _ => SyncTarget::All,
            };
            let report = syncer.sync(sync_target, optimize_bun)?;
            syncer.print_report(&report);
        }
        Commands::WorkIq { action } => {
            let client = WorkIqClient::new();
            match action {
                WorkIqAction::Status => {
                    let status = client.get_status().await?;
                    workiq::print_status(&status);
                }
                WorkIqAction::Search { query } => {
                    let items = client.search(&query).await?;
                    println!("Work IQ Arama Sonuçları ('{}'):", query);
                    for item in items {
                        println!("• [{}] {} - {}", item.source, item.title, item.summary);
                    }
                }
                WorkIqAction::Agenda => {
                    let items = client.get_agenda().await?;
                    println!("Work IQ Takvim & Etkinlikler:");
                    for item in items {
                        println!("• {} ({:?}) - {}", item.title, item.timestamp, item.summary);
                    }
                }
                WorkIqAction::RecentFiles => {
                    let items = client.get_recent_files().await?;
                    println!("Work IQ Son Belgeler (M365 / OneDrive):");
                    for item in items {
                        println!("• {} - {}", item.title, item.summary);
                    }
                }
            }
        }
    }

    Ok(())
}
