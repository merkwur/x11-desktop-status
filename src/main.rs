use serde::Serialize;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use x11rb::atom_manager;


#[derive(Serialize)]
struct WindowInfo {
    id: u32,
    class: String,
    title: String,
    active: bool,
    workspace: Option<u32>, // ← NEW
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
        _NET_WM_CLASS,
        UTF8_STRING,
        WM_NAME,
        _NET_ACTIVE_WINDOW,
        _NET_DESKTOP_NAMES,
        _NET_CURRENT_DESKTOP,
        _NET_WM_DESKTOP,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // establish connection
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atoms = Atoms::new(&conn).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;

    // get workspaces
    let names_reply = conn.get_property(false, root, atoms._NET_DESKTOP_NAMES, atoms.UTF8_STRING, 0, u32::MAX)
        .map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
    let names_str = String::from_utf8(names_reply.value).map_err(|e| e.to_string())?;
    let workspaces = names_str
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    

    // get current workspace 
    let current_workspace = conn
        .get_property(false, root, atoms._NET_CURRENT_DESKTOP, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .value32()
        .ok_or("Could not read current workspace")? 
        .next();


    // get all the open window ids
    let window_ids = conn.get_property(false, root, atoms._NET_CLIENT_LIST, AtomEnum::WINDOW, 0, u32::MAX)
        .map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?
        .value32().ok_or("Failed to parse window list").map_err(|e| e.to_string())?
        .collect::<Vec<_>>();

    // get active window id
    let active_window = conn.get_property(false, root, atoms._NET_ACTIVE_WINDOW, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?
        .value32().and_then(|mut v| v.next()).unwrap_or(0);

    // loop over window ids
    let mut windows = Vec::new();
    for win in window_ids {

        // get title of the window  
        let title = conn.get_property(false, win, atoms._NET_WM_NAME, atoms.UTF8_STRING, 0, u32::MAX)
            .ok()
            .and_then(|r| r.reply().ok())
            .and_then(|r| String::from_utf8(r.value).ok())
            .or_else(|| {
                conn.get_property(false, win, AtomEnum::WM_NAME, AtomEnum::STRING, 0, u32::MAX)
                    .ok()
                    .and_then(|r| r.reply().ok())
                    .and_then(|r| String::from_utf8(r.value).ok())
            })
            .unwrap_or_else(|| "<no name>".to_string());
        
        // get which workspace that window resides
        let workspace: Option<u32> = match conn.get_property(false, win, atoms._NET_WM_DESKTOP, AtomEnum::CARDINAL, 0, 1) {
            Ok(cookie) => match cookie.reply() {
                Ok(reply) => reply.value32().and_then(|mut it| it.next()),
                Err(_) => None,
            },
            Err(_) => None,
        };

        // get the class of the window
        let class = conn
            .get_property(false, win, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, u32::MAX)
            .ok()
            .and_then(|r| r.reply().ok())
            .and_then(|r| {
                let raw = r.value;
                let parts: Vec<&str> = raw.split(|&b| b == 0)
                    .filter_map(|s| std::str::from_utf8(s).ok())
                    .collect();
                parts.get(1).map(|s| s.to_string())
            })
            .unwrap_or_else(|| "<no class>".to_string());

        // populate WindowInfo
        windows.push(WindowInfo {
            id: win,
            class,
            title,
            active: win == active_window,
            workspace,
        });
    }


    // populate the DesktopState
    let state = DesktopState {
        workspaces,
        current_workspace,
        windows,
    };

    // Serialize to JSON and print
    let json = serde_json::to_string_pretty(&state)?;
    println!("{}", json);
    Ok(())
}

