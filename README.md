# 🎵 spotify-cli

Um cliente TUI para o Spotify direto no terminal, escrito em Rust.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- ▶ Controle de playback (play, pause, próxima, anterior)
- 📋 Navegação de playlists
- 🔍 Busca de músicas em tempo real
- 🔊 Controle de volume
- ⇌ Shuffle e repeat
- ⌨️ Navegação estilo vim (j/k)
- 🔐 Auth OAuth2 PKCE (sem client secret exposto)

## Instalação

### Pré-requisitos

- Rust 1.75+ — [instalar](https://rustup.rs)
- Conta Spotify Premium
- App registrado no [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)

### Build

```bash
git clone https://github.com/eng-jomanoel/spotify-cli
cd spotify-cli
cargo build --release
```

O binário fica em `./target/release/spotify-cli`.

### Configuração

Na primeira execução, o programa cria o arquivo de config automaticamente:

```
~/.config/spotify-cli/config.toml
```

Edite com suas credenciais:

```toml
client_id = "seu_client_id"
client_secret = "seu_client_secret"
redirect_port = 8888
```

> No Spotify Developer Dashboard, adicione `http://localhost:8888/callback` como Redirect URI.

### Executar

```bash
cargo run --release
# ou após instalar:
spotify-cli
```

Na primeira execução, o browser abre para autenticação. Após autorizar, o token é salvo em `~/.config/spotify-cli/tokens.json`.

## Atalhos

| Tecla | Ação |
|-------|------|
| `j` / `↓` | Navegar para baixo |
| `k` / `↑` | Navegar para cima |
| `h` / `←` | Ir para painel esquerdo |
| `l` / `→` | Ir para painel direito |
| `Tab` | Alternar painéis |
| `Enter` | Tocar música selecionada |
| `Space` | Play / Pause |
| `n` / `>` | Próxima faixa |
| `p` / `<` | Faixa anterior |
| `+` / `=` | Aumentar volume |
| `-` | Diminuir volume |
| `s` | Toggle shuffle |
| `r` | Toggle repeat |
| `/` | Buscar músicas |
| `R` | Atualizar dados |
| `Esc` | Sair do modo busca |
| `q` | Sair |

## Estrutura do projeto

```
src/
├── main.rs      # Entry point
├── app.rs       # Estado global e loop principal
├── auth.rs      # OAuth2 PKCE flow
├── config.rs    # Config file (~/.config/spotify-cli/)
├── events.rs    # Keyboard events e actions
├── spotify.rs   # Spotify Web API client
└── ui.rs        # Rendering com ratatui
```

## Roadmap

- [ ] Queue management
- [ ] Liked Songs como playlist
- [ ] Album art (sixel/block chars)
- [ ] Letras via API externa
- [ ] `cargo install` support

## License

MIT
