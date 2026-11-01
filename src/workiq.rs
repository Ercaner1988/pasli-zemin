//! Paslı Zemin Windows Work IQ Entegrasyon Modülü
//!
//! Microsoft 365, Microsoft Graph ve Windows Work IQ bağlamını (E-postalar, Takvim,
//! OneDrive/SharePoint belgeleri, Teams sohbetleri ve Kurumsal Semantik Arama)
//! 4 AI arayüzüne (Claude Code, Claude Desktop, OpenAI Codex, Antigravity)
//! hem CLI hem de yerel MCP araçları olarak sunar.

use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkIqStatus {
    pub authenticated: bool,
    pub user_display_name: Option<String>,
    pub user_principal_name: Option<String>,
    pub auth_source: String,
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkIqItem {
    pub id: String,
    pub title: String,
    pub source: String, // "Email", "Calendar", "Document", "Teams"
    pub summary: String,
    pub web_url: Option<String>,
    pub timestamp: Option<String>,
}

pub struct WorkIqClient {
    base_url: String,
    token: Option<String>,
}

impl Default for WorkIqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkIqClient {
    pub fn new() -> Self {
        // Öncelik sırası: WORKIQ_TOKEN -> AZURE_TOKEN -> MS_GRAPH_TOKEN
        let token = env::var("WORKIQ_TOKEN")
            .or_else(|_| env::var("AZURE_TOKEN"))
            .or_else(|_| env::var("MS_GRAPH_TOKEN"))
            .ok();

        Self {
            base_url: env::var("WORKIQ_ENDPOINT")
                .unwrap_or_else(|_| "https://graph.microsoft.com/v1.0".to_string()),
            token,
        }
    }

    pub fn with_token(token: String) -> Self {
        Self {
            base_url: "https://graph.microsoft.com/v1.0".to_string(),
            token: Some(token),
        }
    }

    pub fn has_token(&self) -> bool {
        self.token.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false)
    }

    pub async fn get_status(&self) -> Result<WorkIqStatus> {
        let auth_source = if env::var("WORKIQ_TOKEN").is_ok() {
            "WORKIQ_TOKEN (Çevre Değişkeni)"
        } else if env::var("AZURE_TOKEN").is_ok() {
            "AZURE_TOKEN (Çevre Değişkeni)"
        } else if env::var("MS_GRAPH_TOKEN").is_ok() {
            "MS_GRAPH_TOKEN (Çevre Değişkeni)"
        } else {
            "Giriş Yapılmadı (Yerel Çevrimdışı / Hazırlık Modu)"
        };

        if let Some(ref token) = self.token {
            let client = reqwest::Client::new();
            let res = client
                .get(format!("{}/me", self.base_url))
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/json")
                .send()
                .await;

            if let Ok(response) = res {
                if response.status().is_success() {
                    let val: Value = response.json().await.unwrap_or_default();
                    return Ok(WorkIqStatus {
                        authenticated: true,
                        user_display_name: val.get("displayName").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        user_principal_name: val.get("userPrincipalName").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        auth_source: auth_source.to_string(),
                        endpoints: vec![
                            "Microsoft Graph v1.0 (/me)".into(),
                            "Microsoft 365 Mail (/me/messages)".into(),
                            "Microsoft 365 Calendar (/me/events)".into(),
                            "OneDrive Recent (/me/drive/recent)".into(),
                            "Enterprise Search (/search/query)".into(),
                        ],
                    });
                }
            }
        }

        Ok(WorkIqStatus {
            authenticated: false,
            user_display_name: None,
            user_principal_name: None,
            auth_source: auth_source.to_string(),
            endpoints: vec![
                "https://graph.microsoft.com/v1.0 (Hazır, jeton bekleniyor)".into(),
                "Windows Work IQ Bridge".into(),
            ],
        })
    }

    pub async fn search(&self, query: &str) -> Result<Vec<WorkIqItem>> {
        if !self.has_token() {
            // Çevrimdışı simülasyon / hazırlık yanıtı
            return Ok(vec![
                WorkIqItem {
                    id: "mock-1".into(),
                    title: format!("Work IQ Kurumsal Arama: '{}'", query),
                    source: "Microsoft 365".into(),
                    summary: "Work IQ jetonu girilmediğinde yerel köprü hazırda bekler. WORKIQ_TOKEN çevre değişkeni tanımlandığında canlı Graph API yanıtı döner.".into(),
                    web_url: Some("https://portal.office.com".into()),
                    timestamp: Some(chrono::Local::now().to_rfc3339()),
                }
            ]);
        }

        let token = self.token.as_ref().unwrap();
        let client = reqwest::Client::new();

        let search_payload = json!({
            "requests": [
                {
                    "entityTypes": ["message", "driveItem", "event"],
                    "query": {
                        "queryString": query
                    },
                    "from": 0,
                    "size": 10
                }
            ]
        });

        let res = client
            .post(format!("{}/search/query", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&search_payload)
            .send()
            .await
            .context("Work IQ arama isteği gönderilemedi")?;

        if !res.status().is_success() {
            let err_txt = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Work IQ API hatası: {}", err_txt));
        }

        let val: Value = res.json().await.context("Arama JSON çıktısı çözümlenemedi")?;
        let mut results = Vec::new();

        if let Some(hits_containers) = val["value"][0]["hitsContainers"].as_array() {
            for container in hits_containers {
                if let Some(hits) = container["hits"].as_array() {
                    for hit in hits {
                        let id = hit["hitId"].as_str().unwrap_or("").to_string();
                        let resource = &hit["resource"];
                        let title = resource["name"]
                            .as_str()
                            .or_else(|| resource["subject"].as_str())
                            .unwrap_or("Başlıksız")
                            .to_string();
                        let summary = hit["summary"].as_str().unwrap_or("").to_string();
                        let web_url = resource["webUrl"].as_str().map(|s| s.to_string());
                        let source = hit["resource"]["@odata.type"]
                            .as_str()
                            .unwrap_or("Microsoft 365")
                            .replace("#microsoft.graph.", "");

                        results.push(WorkIqItem {
                            id,
                            title,
                            source,
                            summary,
                            web_url,
                            timestamp: None,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn get_agenda(&self) -> Result<Vec<WorkIqItem>> {
        if !self.has_token() {
            return Ok(vec![WorkIqItem {
                id: "agenda-demo".into(),
                title: "Work IQ Takvim Köprüsü".into(),
                source: "Calendar".into(),
                summary: "Canlı takvim ve etkinlikleri çekmek için WORKIQ_TOKEN gereklidir.".into(),
                web_url: None,
                timestamp: Some(chrono::Local::now().to_rfc3339()),
            }]);
        }

        let token = self.token.as_ref().unwrap();
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/me/events?$top=10&$orderby=start/dateTime desc", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .send()
            .await?;

        let val: Value = res.json().await?;
        let mut items = Vec::new();

        if let Some(events) = val["value"].as_array() {
            for ev in events {
                items.push(WorkIqItem {
                    id: ev["id"].as_str().unwrap_or("").to_string(),
                    title: ev["subject"].as_str().unwrap_or("Konusuz Toplantı").to_string(),
                    source: "Calendar".into(),
                    summary: ev["bodyPreview"].as_str().unwrap_or("").to_string(),
                    web_url: ev["webLink"].as_str().map(|s| s.to_string()),
                    timestamp: ev["start"]["dateTime"].as_str().map(|s| s.to_string()),
                });
            }
        }

        Ok(items)
    }

    pub async fn get_recent_files(&self) -> Result<Vec<WorkIqItem>> {
        if !self.has_token() {
            return Ok(vec![WorkIqItem {
                id: "recent-files-demo".into(),
                title: "Work IQ Dosya Köprüsü (OneDrive/SharePoint)".into(),
                source: "Document".into(),
                summary: "Son düzenlenen M365 belgelerini listelemek için WORKIQ_TOKEN gereklidir.".into(),
                web_url: None,
                timestamp: Some(chrono::Local::now().to_rfc3339()),
            }]);
        }

        let token = self.token.as_ref().unwrap();
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/me/drive/recent?$top=10", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .send()
            .await?;

        let val: Value = res.json().await?;
        let mut items = Vec::new();

        if let Some(files) = val["value"].as_array() {
            for f in files {
                items.push(WorkIqItem {
                    id: f["id"].as_str().unwrap_or("").to_string(),
                    title: f["name"].as_str().unwrap_or("Belge").to_string(),
                    source: "OneDrive".into(),
                    summary: format!("Boyut: {} bayt", f["size"].as_u64().unwrap_or(0)),
                    web_url: f["webUrl"].as_str().map(|s| s.to_string()),
                    timestamp: f["lastModifiedDateTime"].as_str().map(|s| s.to_string()),
                });
            }
        }

        Ok(items)
    }
}

pub fn print_status(status: &WorkIqStatus) {
    println!("{}", "╔═══════════════════════════════════════════════════════════╗".cyan());
    println!("║             {}            ║", "WINDOWS WORK IQ DURUM BİLGİSİ".bold().yellow());
    println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
    let auth_str = if status.authenticated {
        "BAĞLI (Aktif)".green().bold()
    } else {
        "BEKLEMEDE (Jeton Yok)".yellow()
    };
    println!("║ Kimlik Doğrulama:  {:<37} ║", auth_str);
    if let Some(ref name) = status.user_display_name {
        println!("║ Kullanıcı Adı:     {:<37} ║", name.white().bold());
    }
    if let Some(ref upn) = status.user_principal_name {
        println!("║ E-posta / UPN:     {:<37} ║", upn.cyan());
    }
    println!("║ Jeton Kaynağı:     {:<37} ║", status.auth_source.dimmed());
    println!("{}", "╠═══════════════════════════════════════════════════════════╣".cyan());
    println!("║ Desteklenen Work IQ Uç Noktaları:                         ║");
    for ep in &status.endpoints {
        println!("║   • {:<53} ║", ep.dimmed());
    }
    println!("{}", "╚═══════════════════════════════════════════════════════════╝".cyan());
}
