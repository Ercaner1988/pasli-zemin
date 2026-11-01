#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pasli_zemin::run_cli().await
}
