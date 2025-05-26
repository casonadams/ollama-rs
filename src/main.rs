mod app;
mod config;
mod highlight;
mod ollama;
mod tui;

use crate::app::App;
use crate::ollama::stream_to_ollama;
use crate::tui::run_ui;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = crate::config::load_config() {
        eprintln!("failed to initialize config: {}", e);
    }

    let mut app = App::new();
    let cancel_token = CancellationToken::new();

    {
        let ct = cancel_token.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            ct.cancel();
        });
    }

    run_ui(
        &mut app,
        cancel_token.clone(),
        move |input: String, ct, tx| async move {
            stream_to_ollama(input, ct, tx).await;
        },
    )
    .await?;

    Ok(())
}
