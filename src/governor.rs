//! Paslı Zemin Donanım Bekçisi (Governor)
//!
//! Windows 11 / AMD Ryzen ortamında çoklu AI asistanları (Claude Code, Claude Desktop,
//! OpenAI Codex, Google Antigravity) ve ağır derleme süreçleri (cargo/rustc) aynı anda
//! çalışırken donmaları, takılmaları ve bellek şişmesini (OOM / thrashing) önler.
//!
//! Özellikler:
//! - Ön plan etkileşimli arayüzler (Claude, Codex, Antigravity) için ABOVE_NORMAL öncelik ve çekirdek ayrımı
//! - Arka plan derleme/işleme araçları (cargo, rustc, node, bun) için BELOW_NORMAL öncelik ve arka çekirdek maskesi
//! - Bellek temizliği (EmptyWorkingSet): Atıl Electron ve Node/Bun çalışanlarının RAM yükünü boşaltma
//! - Sürekli nöbetçi (daemon) veya tek seferlik (trim) çalıştırma

use anyhow::Result;
use colored::*;
use std::collections::HashSet;
use std::time::Duration;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows_sys::Win32::System::ProcessStatus::{EmptyWorkingSet, K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    OpenProcess, SetProcessAffinityMask, SetPriorityClass,
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
    PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION, PROCESS_VM_READ, PROCESS_SET_QUOTA,
};

#[derive(Debug, Clone)]
pub struct GovernorTarget {
    pub name: String,
    pub is_foreground: bool,
    pub is_background_builder: bool,
    pub is_mcp: bool,
}

#[derive(Debug, Default)]
pub struct GovernorStats {
    pub scanned_processes: usize,
    pub trimmed_processes: usize,
    pub boosted_processes: usize,
    pub throttled_processes: usize,
    pub memory_freed_bytes: u64,
}

pub struct Governor {
    pub foreground_names: HashSet<String>,
    pub background_names: HashSet<String>,
    pub mcp_names: HashSet<String>,
    pub foreground_affinity_mask: usize,
    pub background_affinity_mask: usize,
}

impl Default for Governor {
    fn default() -> Self {
        let mut foreground = HashSet::new();
        for name in [
            "claude.exe",
            "codex.exe",
            "antigravity.exe",
            "code.exe",
            "windowsterminal.exe",
        ] {
            foreground.insert(name.to_lowercase());
        }

        let mut background = HashSet::new();
        for name in [
            "cargo.exe",
            "rustc.exe",
            "link.exe",
            "cl.exe",
            "git.exe",
        ] {
            background.insert(name.to_lowercase());
        }

        let mut mcp = HashSet::new();
        for name in [
            "agent-reach-mcp.exe",
            "nazar-mcp.exe",
            "ibnunnedim.exe",
            "kesfuzzunun-sunucu.exe",
            "tikel-mcp.exe",
            "open-notebook-mcp.exe",
            "mcp-remote-bun.exe",
            "claude-code-setup.exe",
            "pasli-zemin.exe",
            "node.exe",
            "bun.exe",
        ] {
            mcp.insert(name.to_lowercase());
        }

        // AMD Ryzen 5 7430U (6 çekirdek / 12 iş parçacığı):
        // Ön plan çekirdekleri: 0..5 (maske: 0x3F = 0b000000111111)
        // Arka plan çekirdekleri: 6..11 (maske: 0xFC0 = 0b111111000000)
        Self {
            foreground_names: foreground,
            background_names: background,
            mcp_names: mcp,
            foreground_affinity_mask: 0x03F,
            background_affinity_mask: 0xFC0,
        }
    }
}

impl Governor {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(windows)]
    pub fn scan_and_tune(&self, trim_ram: bool) -> Result<GovernorStats> {
        let mut stats = GovernorStats::default();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(anyhow::anyhow!("CreateToolhelp32Snapshot başarısız oldu"));
            }

            let mut entry: PROCESSENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

            if Process32First(snapshot, &mut entry) != 0 {
                loop {
                    let pid = entry.th32ProcessID;
                    let exe_name = {
                        let bytes: Vec<u8> = entry
                            .szExeFile
                            .iter()
                            .take_while(|&&c| c != 0)
                            .map(|&c| c as u8)
                            .collect();
                        String::from_utf8_lossy(&bytes).to_lowercase()
                    };

                    stats.scanned_processes += 1;

                    let is_fg = self.foreground_names.contains(&exe_name);
                    let is_bg = self.background_names.contains(&exe_name);
                    let is_mcp = self.mcp_names.contains(&exe_name);

                    if is_fg || is_bg || is_mcp {
                        if let Ok((trimmed, freed, boosted, throttled)) =
                            self.tune_process(pid, is_fg, is_bg, is_mcp, trim_ram)
                        {
                            if trimmed {
                                stats.trimmed_processes += 1;
                                stats.memory_freed_bytes += freed;
                            }
                            if boosted {
                                stats.boosted_processes += 1;
                            }
                            if throttled {
                                stats.throttled_processes += 1;
                            }
                        }
                    }

                    if Process32Next(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
        }

        Ok(stats)
    }

    #[cfg(windows)]
    fn tune_process(
        &self,
        pid: u32,
        is_fg: bool,
        is_bg: bool,
        is_mcp: bool,
        trim_ram: bool,
    ) -> Result<(bool, u64, bool, bool)> {
        let mut trimmed = false;
        let mut freed_bytes: u64 = 0;
        let mut boosted = false;
        let mut throttled = false;

        unsafe {
            let handle: HANDLE = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION | PROCESS_VM_READ | PROCESS_SET_QUOTA,
                0,
                pid,
            );

            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                return Ok((false, 0, false, false));
            }

            // Bellek bilgisi ölçümü (önce)
            let mut mem_before: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            mem_before.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            let got_mem = K32GetProcessMemoryInfo(
                handle,
                &mut mem_before,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ) != 0;

            // Öncelik ve Çekirdek Ayarları
            if is_fg {
                // Etkileşimli ön plan: Claude, Codex, Antigravity
                SetPriorityClass(handle, ABOVE_NORMAL_PRIORITY_CLASS);
                boosted = true;
            } else if is_bg {
                // Ağır derleme: cargo, rustc, link
                SetPriorityClass(handle, BELOW_NORMAL_PRIORITY_CLASS);
                // Arka plan çekirdeklerine sınırla (0xFC0)
                SetProcessAffinityMask(handle, self.background_affinity_mask);
                throttled = true;
            } else if is_mcp {
                // MCP sunucuları: Normal öncelik, fakat bellek sıkışması engellenir
                SetPriorityClass(handle, NORMAL_PRIORITY_CLASS);
            }

            // Bellek Çalışma Kümesi Temizliği (EmptyWorkingSet)
            if trim_ram && (is_mcp || is_fg || is_bg) {
                if EmptyWorkingSet(handle) != 0 {
                    trimmed = true;

                    if got_mem {
                        let mut mem_after: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                        mem_after.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                        if K32GetProcessMemoryInfo(
                            handle,
                            &mut mem_after,
                            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                        ) != 0 {
                            if mem_before.WorkingSetSize > mem_after.WorkingSetSize {
                                freed_bytes =
                                    (mem_before.WorkingSetSize - mem_after.WorkingSetSize) as u64;
                            }
                        }
                    }
                }
            }

            CloseHandle(handle);
        }

        Ok((trimmed, freed_bytes, boosted, throttled))
    }

    #[cfg(not(windows))]
    pub fn scan_and_tune(&self, _trim_ram: bool) -> Result<GovernorStats> {
        Ok(GovernorStats::default())
    }

    pub fn print_report(&self, stats: &GovernorStats) {
        println!("{}", "╔═══════════════════════════════════════════════════════════╗".cyan());
        println!("║       {}         ║", "PASLI ZEMIN DONANIM BEKCISI (QoS & RAM) RAPORU".bold().yellow());
        println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
        println!("║ Taranan Süreç Sayısı: {:<35} ║", stats.scanned_processes.to_string().white());
        println!("║ Hızlandırılan Ön Plan (ABOVE_NORMAL): {:<18} ║", stats.boosted_processes.to_string().green().bold());
        println!("║ Dizginlenen Derleme (BELOW_NORMAL):  {:<18} ║", stats.throttled_processes.to_string().blue());
        println!("║ RAM'i Temizlenen Süreç Sayısı:        {:<18} ║", stats.trimmed_processes.to_string().magenta());
        let mb_freed = stats.memory_freed_bytes as f64 / (1024.0 * 1024.0);
        println!("║ Fiziksel RAM'e İade Edilen:          {:<18} ║", format!("{:.2} MB", mb_freed).green().bold());
        println!("{}", "╚═══════════════════════════════════════════════════════════╝".cyan());
    }

    pub fn run_daemon(&self, interval_secs: u64) -> Result<()> {
        println!(
            "{} Paslı Zemin Donanım Bekçisi nöbette (her {} saniyede bir)...",
            "🛡️".green(),
            interval_secs
        );
        println!("Çıkmak için Ctrl+C tuşlayın.\n");

        loop {
            match self.scan_and_tune(true) {
                Ok(stats) => {
                    let now = chrono::Local::now().format("%H:%M:%S").to_string();
                    let mb_freed = stats.memory_freed_bytes as f64 / (1024.0 * 1024.0);
                    println!(
                        "[{}] 🛡️ {} ön plan hızlandırıldı, {} derleme dizginlendi, {} süreçten {:.1} MB RAM kurtarıldı.",
                        now.dimmed(),
                        stats.boosted_processes,
                        stats.throttled_processes,
                        stats.trimmed_processes,
                        mb_freed
                    );
                }
                Err(e) => {
                    eprintln!("{} Bekçi hatası: {}", "✗".red(), e);
                }
            }
            std::thread::sleep(Duration::from_secs(interval_secs));
        }
    }
}
