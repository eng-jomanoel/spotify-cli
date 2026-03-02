use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use crate::app::{App, Mode, Panel};

const GREEN: Color = Color::Rgb(29, 185, 84);
const DARK_GREEN: Color = Color::Rgb(20, 130, 60);
const DIM: Color = Color::Rgb(83, 83, 83);
const SUBTEXT: Color = Color::Rgb(179, 179, 179);
const SURFACE: Color = Color::Rgb(18, 18, 18);
const SURFACE2: Color = Color::Rgb(24, 24, 24);
const WHITE: Color = Color::White;

pub fn render(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout: top bar + content + player bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // content
            Constraint::Length(4), // player
            Constraint::Length(1), // status bar
        ])
        .split(size);

    render_title_bar(f, chunks[0]);
    render_content(f, app, chunks[1]);
    render_player(f, app, chunks[2]);
    render_status_bar(f, app, chunks[3]);
}

fn render_title_bar(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" ♫ spotify-cli  ", Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
        Span::styled("Tab", Style::default().fg(DIM)),
        Span::raw(" switch panel  "),
        Span::styled("j/k", Style::default().fg(DIM)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(DIM)),
        Span::raw(" play  "),
        Span::styled("Space", Style::default().fg(DIM)),
        Span::raw(" pause  "),
        Span::styled("/", Style::default().fg(DIM)),
        Span::raw(" search  "),
        Span::styled("q", Style::default().fg(DIM)),
        Span::raw(" quit"),
    ]))
    .style(Style::default().bg(SURFACE));
    f.render_widget(title, area);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(0)])
        .split(area);

    render_playlists(f, app, chunks[0]);
    render_tracks_or_search(f, app, chunks[1]);
}

fn render_playlists(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.panel == Panel::Playlists;
    let border_style = if is_active {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(DIM)
    };

    let block = Block::default()
        .title(if is_active {
            Span::styled(" 󰲸 Playlists ", Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" 󰲸 Playlists ", Style::default().fg(SUBTEXT))
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(SURFACE));

    let items: Vec<ListItem> = app
        .playlists
        .iter()
        .map(|pl| {
            let count = pl.tracks.total;
            ListItem::new(Line::from(vec![
                Span::styled("  ", Style::default().fg(DIM)),
                Span::raw(pl.name.clone()),
                Span::styled(
                    format!(" ({})", count),
                    Style::default().fg(DIM),
                ),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.playlist_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(GREEN)
                .bg(SURFACE2)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state);
}

fn render_tracks_or_search(f: &mut Frame, app: &App, area: Rect) {
    if app.panel == Panel::Search || app.mode == Mode::Searching {
        render_search(f, app, area);
    } else {
        render_tracks(f, app, area);
    }
}

fn render_tracks(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.panel == Panel::Tracks;
    let border_style = if is_active {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(DIM)
    };

    // Playlist title
    let playlist_name = app
        .playlists
        .get(app.playlist_index)
        .map(|p| p.name.as_str())
        .unwrap_or("Tracks");

    let title_str = format!("  {} ", playlist_name);
    let block = Block::default()
        .title(if is_active {
            Span::styled(title_str, Style::default().fg(GREEN).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(title_str, Style::default().fg(SUBTEXT))
        })
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(SURFACE));

    // Currently playing track ID
    let playing_uri = app
        .playback
        .as_ref()
        .and_then(|pb| pb.item.as_ref())
        .map(|t| t.uri.as_str())
        .unwrap_or("");

    let items: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_current = track.uri == playing_uri;
            let num_style = if is_current {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(DIM)
            };
            let name_style = if is_current {
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(WHITE)
            };

            let indicator = if is_current { "▶" } else { " " };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} {:>3}  ", indicator, i + 1), num_style),
                Span::styled(truncate(&track.name, 28), name_style),
                Span::styled("  ", Style::default()),
                Span::styled(truncate(&track.artist_names(), 22), Style::default().fg(SUBTEXT)),
                Span::styled("  ", Style::default()),
                Span::styled(truncate(&track.album.name, 18), Style::default().fg(DIM)),
                Span::styled(
                    format!("  {}", track.duration_formatted()),
                    Style::default().fg(DIM),
                ),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.track_index));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(SURFACE2).fg(WHITE));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_search(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search input
    let is_searching = app.mode == Mode::Searching;
    let search_style = if is_searching {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(DIM)
    };

    let cursor = if is_searching { "█" } else { "" };
    let search_input = Paragraph::new(Line::from(vec![
        Span::styled(" 🔍 ", Style::default().fg(DIM)),
        Span::styled(&app.search_query, Style::default().fg(WHITE)),
        Span::styled(cursor, Style::default().fg(GREEN)),
    ]))
    .block(
        Block::default()
            .title(if is_searching {
                Span::styled(
                    " Search — type to search, Esc to exit ",
                    Style::default().fg(GREEN),
                )
            } else {
                Span::styled(
                    " Search — press / to start ",
                    Style::default().fg(DIM),
                )
            })
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(search_style)
            .style(Style::default().bg(SURFACE)),
    );

    f.render_widget(search_input, chunks[0]);

    // Results
    let playing_uri = app
        .playback
        .as_ref()
        .and_then(|pb| pb.item.as_ref())
        .map(|t| t.uri.as_str())
        .unwrap_or("");

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .map(|track| {
            let is_current = track.uri == playing_uri;
            ListItem::new(Line::from(vec![
                Span::styled(
                    if is_current { " ▶ " } else { "   " },
                    Style::default().fg(GREEN),
                ),
                Span::styled(truncate(&track.name, 30), if is_current {
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(WHITE)
                }),
                Span::styled("  ", Style::default()),
                Span::styled(truncate(&track.artist_names(), 24), Style::default().fg(SUBTEXT)),
                Span::styled("  ", Style::default()),
                Span::styled(truncate(&track.album.name, 20), Style::default().fg(DIM)),
            ]))
        })
        .collect();

    let title = if app.search_results.is_empty() && !app.search_query.is_empty() {
        format!(" No results for \"{}\" ", app.search_query)
    } else if app.search_results.is_empty() {
        " Results ".to_string()
    } else {
        format!(" {} results ", app.search_results.len())
    };

    let mut state = ListState::default();
    state.select(if app.search_results.is_empty() {
        None
    } else {
        Some(app.search_index)
    });

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(SUBTEXT)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(SURFACE)),
        )
        .highlight_style(Style::default().bg(SURFACE2));

    f.render_stateful_widget(list, area, &mut state);
}

fn render_player(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(DIM))
        .style(Style::default().bg(SURFACE));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(inner);

    // LEFT: now playing
    if let Some(ref pb) = app.playback {
        if let Some(ref track) = pb.item {
            let now_playing = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("  ♫  ", Style::default().fg(GREEN)),
                    Span::styled(
                        truncate(&track.name, 26),
                        Style::default().fg(WHITE).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(truncate(&track.artist_names(), 28), Style::default().fg(SUBTEXT)),
                ]),
            ]);
            f.render_widget(now_playing, chunks[0]);
        } else {
            let idle = Paragraph::new("  Nothing playing")
                .style(Style::default().fg(DIM));
            f.render_widget(idle, chunks[0]);
        }

        // CENTER: controls + progress
        let center_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(chunks[1]);

        // Controls
        let shuffle_color = if pb.shuffle_state { GREEN } else { DIM };
        let repeat_color = match pb.repeat_state.as_str() {
            "off" => DIM,
            _ => GREEN,
        };
        let play_icon = if pb.is_playing { "⏸" } else { "▶" };
        let repeat_icon = match pb.repeat_state.as_str() {
            "track" => "↺¹",
            _ => "↺",
        };

        let controls = Paragraph::new(Line::from(vec![
            Span::styled(format!("  ⇌  "), Style::default().fg(shuffle_color)),
            Span::styled("⏮  ", Style::default().fg(WHITE)),
            Span::styled(format!(" {} ", play_icon), Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
            Span::styled("  ⏭  ", Style::default().fg(WHITE)),
            Span::styled(format!("{}  ", repeat_icon), Style::default().fg(repeat_color)),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(controls, center_chunks[0]);

        // Progress bar
        if let (Some(progress_ms), Some(ref track)) = (pb.progress_ms, &pb.item) {
            let pct = (progress_ms as f64 / track.duration_ms as f64).min(1.0) * 100.0;
            let elapsed = format_ms(progress_ms);
            let total = format_ms(track.duration_ms);

            let gauge = Gauge::default()
                .gauge_style(Style::default().fg(GREEN).bg(SURFACE2))
                .ratio(pct / 100.0)
                .label(format!("{} / {}", elapsed, total));
            f.render_widget(gauge, center_chunks[1]);
        }

        // RIGHT: volume + device
        let vol = pb
            .device
            .as_ref()
            .and_then(|d| d.volume_percent)
            .unwrap_or(0);
        let device_name = pb
            .device
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or("No device");

        let vol_bar = "█".repeat((vol as usize * 10 / 100).min(10));
        let vol_empty = "░".repeat(10 - vol_bar.len());

        let right = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  🔊 ", Style::default().fg(DIM)),
                Span::styled(&vol_bar, Style::default().fg(GREEN)),
                Span::styled(vol_empty, Style::default().fg(DIM)),
                Span::styled(format!(" {}%", vol), Style::default().fg(SUBTEXT)),
            ]),
            Line::from(vec![
                Span::styled("     ", Style::default()),
                Span::styled(truncate(device_name, 20), Style::default().fg(DIM)),
            ]),
        ]);
        f.render_widget(right, chunks[2]);
    } else {
        let idle = Paragraph::new(
            Line::from(vec![Span::styled(
                "  No active Spotify session. Open Spotify on any device.",
                Style::default().fg(DIM),
            )])
        );
        f.render_widget(idle, inner);
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let msg = app
        .status_msg
        .as_deref()
        .unwrap_or("Ready  •  +/- volume  •  s shuffle  •  r repeat  •  R refresh  •  / search");

    let status = Paragraph::new(Line::from(vec![
        Span::styled(" ", Style::default()),
        Span::styled(msg, Style::default().fg(SUBTEXT)),
    ]))
    .style(Style::default().bg(SURFACE));

    f.render_widget(status, area);
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!("{}…", chars[..max - 1].iter().collect::<String>())
    }
}

fn format_ms(ms: u64) -> String {
    let total = ms / 1000;
    format!("{}:{:02}", total / 60, total % 60)
}
