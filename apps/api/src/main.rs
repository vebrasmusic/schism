#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::var("SCHISM_API_ADDR").unwrap_or_else(|_| "127.0.0.1:3001".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    eprintln!("schism api listening on http://{addr}");
    axum::serve(listener, api::router()).await?;

    Ok(())
}
