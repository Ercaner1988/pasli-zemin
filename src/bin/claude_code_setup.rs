//! Geriye dönük uyumluluk ikilisi (claude-code-setup)
//! Mevcut konfigürasyonların kırılmaması için pasli_zemin::run_cli'ye yönlendirir.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pasli_zemin::run_cli().await
}
