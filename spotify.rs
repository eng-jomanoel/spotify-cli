use anyhow::{bail, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::auth::TokenData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration_ms: u64,
    pub uri: String,
}

impl Track {
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn duration_formatted(&self) -> String {
        let total = self.duration_ms / 1000;
        format!("{}:{:02}", total / 60, total % 60)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub tracks: PlaylistTracksRef,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistTracksRef {
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub progress_ms: Option<u64>,
    pub item: Option<Track>,
    pub shuffle_state: bool,
    pub repeat_state: String, // "off", "track", "context"
    pub device: Option<Device>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub volume_percent: Option<u8>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct SpotifyClient {
    client: Client,
    token: TokenData,
}

impl SpotifyClient {
    pub fn new(token: TokenData) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token.access_token)
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let resp = self
            .client
            .get(url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if resp.status() == StatusCode::NO_CONTENT {
            bail!("No content");
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            bail!("API error {}: {}", status, text);
        }

        Ok(resp.json().await?)
    }

    async fn put(&self, url: &str, body: Option<serde_json::Value>) -> Result<()> {
        let mut req = self
            .client
            .put(url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            req = req.body(b.to_string());
        }

        let resp = req.send().await?;
        if !resp.status().is_success() && resp.status() != StatusCode::NO_CONTENT {
            let status = resp.status();
            let text = resp.text().await?;
            bail!("API error {}: {}", status, text);
        }
        Ok(())
    }

    async fn post(&self, url: &str, body: Option<serde_json::Value>) -> Result<()> {
        let mut req = self
            .client
            .post(url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            req = req.body(b.to_string());
        }

        let resp = req.send().await?;
        if !resp.status().is_success() && resp.status() != StatusCode::NO_CONTENT {
            let status = resp.status();
            let text = resp.text().await?;
            bail!("API error {}: {}", status, text);
        }
        Ok(())
    }

    // ─── Playback ────────────────────────────────────────────────────────────

    pub async fn get_playback(&self) -> Result<Option<PlaybackState>> {
        match self
            .get::<PlaybackState>("https://api.spotify.com/v1/me/player")
            .await
        {
            Ok(state) => Ok(Some(state)),
            Err(e) if e.to_string().contains("No content") => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn play(&self, uri: Option<&str>, context_uri: Option<&str>) -> Result<()> {
        let body = match (uri, context_uri) {
            (Some(u), _) => Some(serde_json::json!({ "uris": [u] })),
            (_, Some(c)) => Some(serde_json::json!({ "context_uri": c })),
            _ => None,
        };
        self.put("https://api.spotify.com/v1/me/player/play", body)
            .await
    }

    pub async fn pause(&self) -> Result<()> {
        self.put("https://api.spotify.com/v1/me/player/pause", None)
            .await
    }

    pub async fn next(&self) -> Result<()> {
        self.post("https://api.spotify.com/v1/me/player/next", None)
            .await
    }

    pub async fn previous(&self) -> Result<()> {
        self.post("https://api.spotify.com/v1/me/player/previous", None)
            .await
    }

    pub async fn seek(&self, position_ms: u64) -> Result<()> {
        self.put(
            &format!(
                "https://api.spotify.com/v1/me/player/seek?position_ms={}",
                position_ms
            ),
            None,
        )
        .await
    }

    pub async fn set_volume(&self, volume: u8) -> Result<()> {
        self.put(
            &format!(
                "https://api.spotify.com/v1/me/player/volume?volume_percent={}",
                volume
            ),
            None,
        )
        .await
    }

    pub async fn set_shuffle(&self, state: bool) -> Result<()> {
        self.put(
            &format!(
                "https://api.spotify.com/v1/me/player/shuffle?state={}",
                state
            ),
            None,
        )
        .await
    }

    pub async fn set_repeat(&self, state: &str) -> Result<()> {
        self.put(
            &format!(
                "https://api.spotify.com/v1/me/player/repeat?state={}",
                state
            ),
            None,
        )
        .await
    }

    // ─── Library ─────────────────────────────────────────────────────────────

    pub async fn get_playlists(&self, limit: u32, offset: u32) -> Result<Vec<Playlist>> {
        #[derive(Deserialize)]
        struct Page {
            items: Vec<Playlist>,
        }
        let url = format!(
            "https://api.spotify.com/v1/me/playlists?limit={}&offset={}",
            limit, offset
        );
        let page: Page = self.get(&url).await?;
        Ok(page.items)
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        #[derive(Deserialize)]
        struct PlaylistTrackItem {
            track: Option<Track>,
        }
        #[derive(Deserialize)]
        struct Page {
            items: Vec<PlaylistTrackItem>,
        }
        let url = format!(
            "https://api.spotify.com/v1/playlists/{}/tracks?limit=50",
            playlist_id
        );
        let page: Page = self.get(&url).await?;
        Ok(page.items.into_iter().filter_map(|i| i.track).collect())
    }

    pub async fn get_liked_songs(&self, limit: u32, offset: u32) -> Result<Vec<Track>> {
        #[derive(Deserialize)]
        struct SavedTrackItem {
            track: Track,
        }
        #[derive(Deserialize)]
        struct Page {
            items: Vec<SavedTrackItem>,
        }
        let url = format!(
            "https://api.spotify.com/v1/me/tracks?limit={}&offset={}",
            limit, offset
        );
        let page: Page = self.get(&url).await?;
        Ok(page.items.into_iter().map(|i| i.track).collect())
    }

    // ─── Search ──────────────────────────────────────────────────────────────

    pub async fn search(&self, query: &str) -> Result<Vec<Track>> {
        #[derive(Deserialize)]
        struct Tracks {
            items: Vec<Track>,
        }
        #[derive(Deserialize)]
        struct SearchResult {
            tracks: Tracks,
        }
        let url = format!(
            "https://api.spotify.com/v1/search?q={}&type=track&limit=20",
            urlencoding_encode(query)
        );
        let result: SearchResult = self.get(&url).await?;
        Ok(result.tracks.items)
    }
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}
