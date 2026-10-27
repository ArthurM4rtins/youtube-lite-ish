use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Rect, Url, Webview,
    WebviewBuilder, WebviewUrl, WindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_opener::OpenerExt;

const YOUTUBE_URL: &str = "https://www.youtube.com/";
const MAIN_WINDOW: &str = "main";
const TABBAR: &str = "tabbar";
const TAB_PREFIX: &str = "tab-";
const TABBAR_HEIGHT: f64 = 38.0;
const APP_TITLE: &str = "YouTube Lite";
const LITE_CSS: &str = include_str!("lite.css");
const INJECT_JS: &str = include_str!("inject.js");

const MIN_SIZE: (f64, f64) = (500.0, 400.0);
const MINI_SIZE: (f64, f64) = (480.0, 270.0);
const MINI_MIN_SIZE: (f64, f64) = (240.0, 135.0);
/// Logical-pixel gap from the right edge and from the bottom (clears the taskbar)
const MINI_MARGIN: (f64, f64) = (24.0, 72.0);

#[derive(Clone, Copy, PartialEq, Default)]
enum Mode
{
    #[default]
    Normal,
    Fullscreen,
    Mini,
}

/// Window bounds to restore when leaving the mini player
struct SavedBounds
{
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    maximized: bool,
}

struct Tab
{
    id: u32,
    title: String,
}

#[derive(Default)]
struct Tabs
{
    list: Vec<Tab>,
    active: Option<u32>,
    next_id: u32,
    mode: Mode,
    saved: Option<SavedBounds>,
}

/// Never hold this lock while calling a window getter: getters wait on the main thread,
/// which may itself be waiting for this lock (resize events)
#[derive(Default)]
struct AppState(Mutex<Tabs>);

#[derive(Serialize)]
struct TabView<'a>
{
    id: u32,
    title: &'a str,
    active: bool,
}

fn tabs(app: &AppHandle) -> MutexGuard<'_, Tabs>
{
    app.state::<AppState>().inner().0.lock().expect("tabs lock")
}

fn tab_label(id: u32) -> String
{
    format!("{TAB_PREFIX}{id}")
}

fn tab_id(label: &str) -> Option<u32>
{
    label.strip_prefix(TAB_PREFIX)?.parse().ok()
}

fn home_url() -> Url
{
    YOUTUBE_URL.parse().expect("valid YouTube URL")
}

fn start_url() -> Url
{
    // Debug builds can open a specific page for testing
    #[cfg(debug_assertions)]
    if let Some(url) = std::env::var("YOUTUBE_LITE_URL").ok().and_then(|u| u.parse().ok())
    {
        return url;
    }
    home_url()
}

fn open_in_browser(app: &AppHandle, url: &str)
{
    if url.starts_with("https://") || url.starts_with("http://")
    {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

fn host_matches(host: &str, domain: &str) -> bool
{
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// YouTube and Google sign-in stay inside the app; other http(s) pages open in the browser
fn is_internal(url: &Url) -> bool
{
    match url.scheme()
    {
        "http" | "https" =>
        {
            let host = url.host_str().unwrap_or_default();
            ["youtube.com", "youtu.be", "google.com", "google.com.br"]
                .iter()
                .any(|domain| host_matches(host, domain))
        }
        _ => true,
    }
}

fn init_script() -> String
{
    let css = serde_json::to_string(LITE_CSS).expect("CSS serializes to a JS string");
    format!("window.__YT_LITE_CSS__ = {css};\n{INJECT_JS}")
}

/// Tab bar on top (hidden in fullscreen/mini), active tab fills the rest, other tabs hidden
fn layout(app: &AppHandle)
{
    let Some(window) = app.get_window(MAIN_WINDOW) else
    {
        return;
    };
    let (Ok(physical), Ok(scale)) = (window.inner_size(), window.scale_factor()) else
    {
        return;
    };
    let size = physical.to_logical::<f64>(scale);
    let (ids, active, mode) = {
        let tabs = tabs(app);
        (tabs.list.iter().map(|tab| tab.id).collect::<Vec<_>>(), tabs.active, tabs.mode)
    };

    let bar = if mode == Mode::Normal { TABBAR_HEIGHT } else { 0.0 };
    if let Some(tabbar) = app.get_webview(TABBAR)
    {
        if bar > 0.0
        {
            let _ = tabbar.set_bounds(Rect {
                position: LogicalPosition::new(0.0, 0.0).into(),
                size: LogicalSize::new(size.width, bar).into(),
            });
            let _ = tabbar.show();
        }
        else
        {
            let _ = tabbar.hide();
        }
    }

    for id in ids
    {
        let Some(webview) = app.get_webview(&tab_label(id)) else
        {
            continue;
        };
        if Some(id) == active
        {
            let _ = webview.set_bounds(Rect {
                position: LogicalPosition::new(0.0, bar).into(),
                size: LogicalSize::new(size.width, (size.height - bar).max(0.0)).into(),
            });
            let _ = webview.show();
        }
        else
        {
            let _ = webview.hide();
        }
    }
}

/// Push the tab list to the tab bar and the active title to the window
fn sync_ui(app: &AppHandle)
{
    let (json, title) = {
        let tabs = tabs(app);
        let views: Vec<TabView> = tabs
            .list
            .iter()
            .map(|tab| TabView { id: tab.id, title: &tab.title, active: Some(tab.id) == tabs.active })
            .collect();
        let title = tabs.list.iter().find(|tab| Some(tab.id) == tabs.active).map(|tab| tab.title.clone());
        (serde_json::to_string(&views).unwrap_or_else(|_| "[]".into()), title)
    };
    if let Some(tabbar) = app.get_webview(TABBAR)
    {
        let _ = tabbar.eval(format!("window.renderTabs && window.renderTabs({json})"));
    }
    if let Some(window) = app.get_window(MAIN_WINDOW)
    {
        let _ = window.set_title(&title.map_or_else(|| APP_TITLE.to_string(), |t| format!("{t} - {APP_TITLE}")));
    }
}

fn refresh(app: &AppHandle)
{
    layout(app);
    sync_ui(app);
    let active = tabs(app).active;
    if let Some(webview) = active.and_then(|id| app.get_webview(&tab_label(id)))
    {
        let _ = webview.set_focus();
    }
}

fn set_tab_title(app: &AppHandle, label: &str, title: String)
{
    let Some(id) = tab_id(label) else
    {
        return;
    };
    {
        let mut tabs = tabs(app);
        let Some(tab) = tabs.list.iter_mut().find(|tab| tab.id == id) else
        {
            return;
        };
        tab.title = title.trim_end_matches(" - YouTube").to_string();
    }
    sync_ui(app);
}

fn youtube_webview(app: &AppHandle, id: u32, url: Url) -> WebviewBuilder<Wry>
{
    let nav_app = app.clone();
    let popup_app = app.clone();
    let title_app = app.clone();
    WebviewBuilder::new(tab_label(id), WebviewUrl::External(url))
        .initialization_script(init_script())
        .on_navigation(move |url|
        {
            if is_internal(url)
            {
                return true;
            }
            open_in_browser(&nav_app, url.as_str());
            false
        })
        // Ctrl+click, middle click and "open in new window" become background tabs
        .on_new_window(move |url, _features|
        {
            let app = popup_app.clone();
            // Creating a webview inside this WebView2 callback is unsafe, so do it from another thread
            std::thread::spawn(move ||
            {
                if !matches!(url.scheme(), "http" | "https")
                {
                    // e.g. about:blank from a browser shortcut: nothing to open
                }
                else if is_internal(&url)
                {
                    let _ = open_tab(&app, url, true);
                }
                else
                {
                    open_in_browser(&app, url.as_str());
                }
            });
            NewWindowResponse::Deny
        })
        .on_document_title_changed(move |webview, title|
        {
            set_tab_title(&title_app, webview.label(), title);
        })
}

fn open_tab(app: &AppHandle, url: Url, background: bool) -> tauri::Result<()>
{
    let Some(window) = app.get_window(MAIN_WINDOW) else
    {
        return Ok(());
    };
    let id = {
        let mut tabs = tabs(app);
        tabs.next_id += 1;
        let id = tabs.next_id;
        tabs.list.push(Tab { id, title: "Carregando...".into() });
        if !background || tabs.active.is_none()
        {
            tabs.active = Some(id);
        }
        id
    };
    window.add_child(youtube_webview(app, id, url), LogicalPosition::new(0.0, 0.0), LogicalSize::new(1.0, 1.0))?;
    refresh(app);
    Ok(())
}

fn close_tab_by_id(app: &AppHandle, id: u32)
{
    let remaining = {
        let mut tabs = tabs(app);
        let Some(index) = tabs.list.iter().position(|tab| tab.id == id) else
        {
            return;
        };
        tabs.list.remove(index);
        if tabs.active == Some(id)
        {
            // Like Chrome: the tab to the right takes over, or the left one if it was the last
            let next = tabs.list.get(index).or_else(|| tabs.list.last()).map(|tab| tab.id);
            tabs.active = next;
        }
        tabs.list.len()
    };
    if let Some(webview) = app.get_webview(&tab_label(id))
    {
        let _ = webview.close();
    }
    if remaining == 0
    {
        app.exit(0);
        return;
    }
    refresh(app);
}

#[tauri::command(async)]
fn open_external(app: AppHandle, url: String)
{
    open_in_browser(&app, &url);
}

#[tauri::command(async)]
fn new_tab(app: AppHandle, url: Option<String>, background: bool) -> tauri::Result<()>
{
    let url = url.and_then(|u| u.parse::<Url>().ok()).filter(is_internal).unwrap_or_else(home_url);
    open_tab(&app, url, background)
}

#[tauri::command(async)]
fn close_tab(app: AppHandle, id: u32)
{
    close_tab_by_id(&app, id);
}

#[tauri::command(async)]
fn close_current(app: AppHandle, webview: Webview)
{
    if let Some(id) = tab_id(webview.label())
    {
        close_tab_by_id(&app, id);
    }
}

#[tauri::command(async)]
fn activate_tab(app: AppHandle, id: u32)
{
    {
        let mut tabs = tabs(&app);
        if !tabs.list.iter().any(|tab| tab.id == id)
        {
            return;
        }
        tabs.active = Some(id);
    }
    refresh(&app);
}

#[tauri::command(async)]
fn cycle_tab(app: AppHandle, offset: i32)
{
    {
        let mut tabs = tabs(&app);
        let count = tabs.list.len() as i32;
        let Some(current) = tabs.active.and_then(|id| tabs.list.iter().position(|tab| tab.id == id)) else
        {
            return;
        };
        let next = (current as i32 + offset).rem_euclid(count) as usize;
        let next_id = tabs.list[next].id;
        tabs.active = Some(next_id);
    }
    refresh(&app);
}

#[tauri::command(async)]
fn set_fullscreen(app: AppHandle, fullscreen: bool)
{
    {
        let mut tabs = tabs(&app);
        if tabs.mode == Mode::Mini
        {
            return;
        }
        tabs.mode = if fullscreen { Mode::Fullscreen } else { Mode::Normal };
    }
    if let Some(window) = app.get_window(MAIN_WINDOW)
    {
        let _ = window.set_fullscreen(fullscreen);
    }
    layout(&app);
}

#[tauri::command(async)]
fn start_drag(app: AppHandle)
{
    if let Some(window) = app.get_window(MAIN_WINDOW)
    {
        let _ = window.start_dragging();
    }
}

#[tauri::command(async)]
fn set_mini(app: AppHandle, on: bool) -> tauri::Result<()>
{
    let Some(window) = app.get_window(MAIN_WINDOW) else
    {
        return Ok(());
    };
    if on
    {
        if tabs(&app).saved.is_none()
        {
            window.set_fullscreen(false)?;
            let bounds = SavedBounds {
                position: window.outer_position()?,
                size: window.inner_size()?,
                maximized: window.is_maximized()?,
            };
            tabs(&app).saved = Some(bounds);
        }
        tabs(&app).mode = Mode::Mini;

        window.unmaximize()?;
        window.set_decorations(false)?;
        window.set_min_size(Some(LogicalSize::new(MINI_MIN_SIZE.0, MINI_MIN_SIZE.1)))?;
        let scale = window.scale_factor()?;
        let size = PhysicalSize::new((MINI_SIZE.0 * scale) as u32, (MINI_SIZE.1 * scale) as u32);
        window.set_size(size)?;
        if let Some(monitor) = window.current_monitor()?
        {
            let origin = monitor.position();
            let screen = monitor.size();
            let x = origin.x + screen.width as i32 - size.width as i32 - (MINI_MARGIN.0 * scale) as i32;
            let y = origin.y + screen.height as i32 - size.height as i32 - (MINI_MARGIN.1 * scale) as i32;
            window.set_position(PhysicalPosition::new(x, y))?;
        }
        window.set_always_on_top(true)?;
    }
    else
    {
        let saved = {
            let mut tabs = tabs(&app);
            tabs.mode = Mode::Normal;
            tabs.saved.take()
        };
        if let Some(bounds) = saved
        {
            window.set_always_on_top(false)?;
            window.set_decorations(true)?;
            window.set_min_size(Some(LogicalSize::new(MIN_SIZE.0, MIN_SIZE.1)))?;
            window.set_size(bounds.size)?;
            window.set_position(bounds.position)?;
            if bounds.maximized
            {
                window.maximize()?;
            }
            window.set_focus()?;
        }
    }
    layout(&app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run()
{
    tauri::Builder::default()
        // Must be first: a second launch just focuses the running instance
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd|
        {
            if let Some(window) = app.get_window(MAIN_WINDOW)
            {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_external,
            new_tab,
            close_tab,
            close_current,
            activate_tab,
            cycle_tab,
            set_fullscreen,
            start_drag,
            set_mini
        ])
        .setup(|app|
        {
            let handle = app.handle().clone();
            let mut builder = WindowBuilder::new(app, MAIN_WINDOW)
                .title(APP_TITLE)
                .inner_size(1400.0, 850.0)
                .min_inner_size(MIN_SIZE.0, MIN_SIZE.1);
            // Set the icon on the window itself so the taskbar never shows a stale cached shell icon
            if let Some(icon) = app.default_window_icon()
            {
                builder = builder.icon(icon.clone())?;
            }
            let window = builder.build()?;

            let resize_app = handle.clone();
            window.on_window_event(move |event|
            {
                if matches!(event, WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. })
                {
                    layout(&resize_app);
                }
            });

            let tabbar_app = handle.clone();
            let tabbar = WebviewBuilder::new(TABBAR, WebviewUrl::App("index.html".into())).on_page_load(move |_, payload|
            {
                if payload.event() == PageLoadEvent::Finished
                {
                    sync_ui(&tabbar_app);
                }
            });
            window.add_child(tabbar, LogicalPosition::new(0.0, 0.0), LogicalSize::new(1400.0, TABBAR_HEIGHT))?;

            open_tab(&handle, start_url(), false)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running YouTube Lite");
}
