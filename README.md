# Youtube Lite-ish (Unofficial Wrapper)

A distraction-free YouTube desktop app for Windows: the **official [youtube.com](https://www.youtube.com)** in a lightweight [Tauri](https://tauri.app) window (Microsoft WebView2), with a layer of CSS/JS that strips the page down to what matters.

> **Not affiliated with, endorsed by or connected to YouTube or Google.** YouTube is a trademark of Google LLC. This is a personal, unofficial wrapper. It does not block ads or download videos.

## What changes

- **Side menu:** only Home, Subscriptions and History
- **Watch page:** player, title/channel, description, **only the pinned comment**, and recommendations on the right
- **Shorts are gone** everywhere; a Shorts link opens as a regular video
- **Every new video starts at 2x** (changing the speed by hand sticks until the next video; live streams stay at 1x)
- **Top bar:** no keyboard icon, voice search or "Create" button
- **Account menu:** only Google Account, Switch account, Sign out and Appearance
- **Mini player:** a button next to fullscreen turns the window into a small, borderless, always-on-top player in the bottom-right corner. Drag it by the top strip, resize from the edges; leave with **Esc**, double-click or **Voltar**
- **Tabs:** `+` / **Ctrl+N** new tab, **Ctrl+W** close, **Ctrl+Tab** switch; Ctrl+click or middle-click opens a video in a background tab (it only starts playing when you open the tab)
- **Paste a video link** into the search box and press Enter — or press **Ctrl+V** anywhere outside a text field — to open it directly
- External links open in your default browser

## Install

Download `Youtube-Lite-ish_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it. Sign in with your Google account inside the app.

The installer is not code-signed, so Windows SmartScreen or your antivirus may warn about it. If you prefer, build it yourself (below).

## Tweak it

- `src-tauri/src/lite.css` — what gets hidden
- `src-tauri/src/inject.js` — `DEFAULT_SPEED`, mini player, Shorts, links, tabs shortcuts
- `src-tauri/src/lib.rs` — tabs and mini player size/position (`MINI_SIZE`, `MINI_MARGIN`)

YouTube changes its page often; if something reappears, the selectors in `lite.css` are the place to look.

## Build

Requirements: [Rust](https://rustup.rs) (MSVC toolchain), [Node.js](https://nodejs.org), Visual Studio Build Tools with C++.

```
build.cmd          # installer -> src-tauri\target\release\bundle\nsis\
build.cmd check    # compile only (debug)
```

Debug builds open a specific page when `YOUTUBE_LITE_URL` is set.

## Security

YouTube tabs can only call the commands listed in `src-tauri/capabilities/youtube.json` (open links/tabs, fullscreen, mini player); the local tab bar only those in `capabilities/tabbar.json`. No other Tauri API is exposed to web pages.

## License

[MIT](LICENSE)
