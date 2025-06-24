# Desktop State Logger via [x11rb](https://github.com/psychon/x11rb)

A simple rust script that log desktop state with [serde](https://github.com/serde-rs/serde) as follows:

```rust
DesktopState 
{ 
    workspaces: Vec<String>, 
    current_workspace: u32
    windows: {
        id: u32, 
        class: String, 
        title: String, 
        active: bool,
        workspace: u32,
    }   
}
```


