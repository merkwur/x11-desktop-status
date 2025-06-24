use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::rust_connection::RustConnection;
use x11rb::atom_manager;
use serde::Serialize;




#[derive(Serialize)]
struct WindowInfo {
    id: u32,
    title: String,
    active: bool,
}

#[derive(Serialize)]
struct DesktopState {
    workspaces: Vec<String>,
    current_workspace: Option<u32>,
    windows: Vec<WindowInfo>,
}

atom_manager! {
    Atoms: AtomsCookie {
        _NET_CLIENT_LIST,
        _NET_WM_NAME,
        UTF8_STRING,
        WM_NAME,
        _NET_ACTIVE_WINDOW,
        _NET_DESKTOP_NAMES,
        _NET_CURRENT_DESKTOP,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (conn, screen_num) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atoms = Atoms::new(&conn)?.reply()?;

    // Get workspace names
    let reply = conn.get_property(false, root, atoms._NET_DESKTOP_NAMES, atoms.UTF8_STRING, 0, u32::MAX)?.reply()?;
    let binding = String::from_utf8(reply.value)?;
    let workspace_names = binding
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string()) 
        .collect::<Vec<_>>();


    // Get current workspace
    let current_workspace = conn.get_property(false, root, atoms._NET_CURRENT_DESKTOP, AtomEnum::CARDINAL, 0, 1)?
        .reply()?
        .value32()
        .expect("Failed to get current workspace")
        .next();

    // Get list of windows
    let windows_reply = conn.get_property(false, root, atoms._NET_CLIENT_LIST, AtomEnum::WINDOW, 0, u32::MAX)?
        .reply()?;
    let window_ids = windows_reply.value32().ok_or("Failed to get client list")?.collect::<Vec<_>>();

    // Get active window
    let active_window = conn.get_property(false, root, atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, 0, 1)?
        .reply()?
        .value32()
        .and_then(|mut v| v.next())
        .unwrap_or(0);

    let mut windows = Vec::new();
    for win in window_ids {
        let title = conn.get_property(false, win, atoms._NET_WM_NAME, atoms.UTF8_STRING, 0, u32::MAX)?
            .reply()
            .ok()
            .and_then(|r| String::from_utf8(r.value).ok())
            .or_else(|| {
                conn.get_property(false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)
                    .ok()?
                    .reply()
                    .ok()
                    .and_then(|r| String::from_utf8(r.value).ok())
            })
            .unwrap_or_else(|| "<no name>".to_string());

        windows.push(WindowInfo {
            id: win,
            title,
            active: win == active_window,
        });
    }


    let state = DesktopState {

        workspaces: workspace_names,
        current_workspace,
        windows,
    };

    // Serialize to JSON and print
    let json = serde_json::to_string_pretty(&state)?;
    println!("{}", json);
    Ok(())
}

