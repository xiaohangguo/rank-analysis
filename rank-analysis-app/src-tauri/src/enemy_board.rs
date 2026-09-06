//! Hold Numpad+ to view the current opponents. No hooks or injected code.
use serde_json::Value;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
#[cfg(target_os = "windows")]
static IN_GAME: AtomicBool = AtomicBool::new(false);

static SNAPSHOT: LazyLock<Mutex<Value>> = LazyLock::new(|| Mutex::new(Value::Null));

#[tauri::command]
pub fn enemy_board_snapshot() -> Value {
    SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(not(target_os = "windows"))]
pub fn start(_: &tauri::AppHandle) {}

#[cfg(target_os = "windows")]
pub fn start(app: &tauri::AppHandle) {
    use tauri::{Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
    let state_app = app.clone();
    app.listen("game-state-changed", move |event| {
        if let Ok(value) = serde_json::from_str::<Value>(event.payload()) {
            let active = value["connected"] == true && value["phase"] == "InProgress";
            let was_active = IN_GAME.swap(active, Ordering::Relaxed);
            if active && !was_active {
                let app = state_app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = crate::command::session::get_session_data(app).await {
                        log::warn!("敌方看板首次加载失败: {e}");
                    }
                });
            }
            if !active {
                *SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = Value::Null;
                let _ = state_app.emit_to("enemy-board", "enemy-board-data", Value::Null);
            }
        }
    });
    for event in ["session-basic-info", "session-complete"] {
        let app = app.clone();
        app.clone().listen(event, move |event| {
            if let Ok(value) = serde_json::from_str::<Value>(event.payload()) {
                *SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner()) = value.clone();
                let _ = app.emit_to("enemy-board", "enemy-board-data", value);
            }
        });
    }
    let update_app = app.clone();
    app.listen("session-player-update", move |event| {
        if let Ok(update) = serde_json::from_str::<Value>(event.payload()) {
            let mut snapshot = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(teams) = snapshot["subteams"].as_array_mut() {
                for team in teams {
                    if team["subteamId"] == update["subteamId"] {
                        if let Some(players) = team["players"].as_array_mut() {
                            if let Some(player) = players.iter_mut().find(|p| {
                                p["summoner"]["puuid"] == update["player"]["summoner"]["puuid"]
                            }) {
                                *player = update["player"].clone();
                            }
                        }
                    }
                }
            }
            let value = snapshot.clone();
            drop(snapshot);
            let _ = update_app.emit_to("enemy-board", "enemy-board-data", value);
        }
    });
    // Created on the UI thread, hidden and explicitly non-activating.
    let window = match WebviewWindowBuilder::new(
        app,
        "enemy-board",
        WebviewUrl::App("enemy-board.html".into()),
    )
    .title("敌方看板 · 按住小键盘 +")
    .inner_size(650.0, 430.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .focusable(false)
    .focused(false)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .visible(false)
    .build()
    {
        Ok(w) => w,
        Err(e) => {
            log::error!("敌方看板创建失败: {e}");
            return;
        }
    };
    if let Err(e) = window.set_ignore_cursor_events(true) {
        log::error!("敌方看板鼠标穿透设置失败: {e}");
        let _ = window.destroy();
        return;
    }
    let app = app.clone();
    // Key sampling runs separately from network work so release never waits for LCU.
    std::thread::spawn(move || {
        use winapi::um::winuser::{
            GetAsyncKeyState, GetForegroundWindow, GetWindowThreadProcessId, VK_ADD,
        };
        let mut visible = false;
        let mut foreground_cache = (0u32, false, std::time::Instant::now());
        loop {
            std::thread::sleep(std::time::Duration::from_millis(16));
            if app.get_webview_window("enemy-board").is_none() {
                break;
            }
            let held = unsafe { GetAsyncKeyState(VK_ADD) < 0 };
            let foreground_game = if held {
                let mut pid = 0;
                unsafe {
                    GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
                }
                if pid != foreground_cache.0 || foreground_cache.2.elapsed().as_secs() >= 1 {
                    foreground_cache = (pid, is_game_process(pid), std::time::Instant::now());
                }
                foreground_cache.1
            } else {
                false
            };
            let in_game = IN_GAME.load(Ordering::Relaxed);
            let desired = held && foreground_game && in_game;
            if desired == visible {
                continue;
            }
            visible = desired;
            let ui_app = app.clone();
            let _ = app.run_on_main_thread(move || {
                if let Some(w) = ui_app.get_webview_window("enemy-board") {
                    use winapi::um::winuser::{ShowWindow, SW_HIDE, SW_SHOWNOACTIVATE};
                    let Ok(hwnd) = w.hwnd() else {
                        return;
                    };
                    let still_held = unsafe { GetAsyncKeyState(VK_ADD) < 0 };
                    let mut foreground_pid = 0;
                    unsafe {
                        GetWindowThreadProcessId(GetForegroundWindow(), &mut foreground_pid);
                    }
                    if desired
                        && still_held
                        && IN_GAME.load(Ordering::Relaxed)
                        && is_game_process(foreground_pid)
                    {
                        // Position on the monitor containing the game, including non-primary monitors.
                        use winapi::shared::windef::RECT;
                        use winapi::um::winuser::{GetForegroundWindow, GetWindowRect};
                        let mut rect: RECT = unsafe { std::mem::zeroed() };
                        if unsafe { GetWindowRect(GetForegroundWindow(), &mut rect) } != 0 {
                            let scale = w.scale_factor().unwrap_or(1.0);
                            let x =
                                rect.left + ((rect.right - rect.left) - (650.0 * scale) as i32) / 2;
                            let _ = w.set_position(tauri::PhysicalPosition::new(
                                x,
                                rect.top + (90.0 * scale) as i32,
                            ));
                        }
                        // Explicit no-activation on EVERY press, not just initial window creation.
                        unsafe {
                            ShowWindow(hwnd.0 as _, SW_SHOWNOACTIVATE);
                        }
                    } else {
                        unsafe {
                            ShowWindow(hwnd.0 as _, SW_HIDE);
                        }
                    }
                }
            });
        }
    });
}

#[cfg(target_os = "windows")]
fn is_game_process(pid: u32) -> bool {
    use winapi::um::{
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        tlhelp32::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if handle == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(handle, &mut entry);
        let mut found = false;
        while more != 0 {
            if entry.th32ProcessID == pid {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                found = String::from_utf16_lossy(&entry.szExeFile[..len])
                    .eq_ignore_ascii_case("League of Legends.exe");
                break;
            }
            more = Process32NextW(handle, &mut entry);
        }
        CloseHandle(handle);
        found
    }
}
