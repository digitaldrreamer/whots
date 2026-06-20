#[tokio::main]
async fn main() -> anyhow::Result<()> {
    whots_server::run().await
}
