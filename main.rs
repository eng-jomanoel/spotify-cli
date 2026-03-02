mod app;
mod auth;
mod config;
mod spotify;
mod ui;
mod events;

use anyhow::Result;
use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = config::Config::load()?;

    // Check auth
    let token = auth::ensure_authenticated(&config).await?;

    // Build spotify client
    let spotify = spotify::SpotifyClient::new(token);

    // Run TUI
    let mut app = App::new(spotify);
    app.run().await?;

    Ok(())
}
