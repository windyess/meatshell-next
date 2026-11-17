use super::*;

pub(super) fn wire_tunnel_callbacks(
    window: &AppWindow,
    handles: Rc<RefCell<HashMap<String, SessionHandle>>>,
    store: Rc<RefCell<ConfigStore>>,
    statuses: TabStatuses,
) {
    {
        let handles = handles.clone();
        let store = store.clone();
        let statuses = statuses.clone();
        window.on_tunnel_add(
            move |tab_id, name, kind, bind, bind_port, host, host_port, save, auto_start| {
                let Some(mut forward) = parse_tunnel_form(
                    &kind, &name, &bind, &bind_port, &host, &host_port, auto_start,
                ) else {
                    return;
                };
                if !handles.borrow().contains_key(tab_id.as_str()) {
                    return;
                }
                let id = uuid::Uuid::new_v4().to_string();
                forward.id = if save { id.clone() } else { String::new() };
                let tunnel_id = if save {
                    format!("saved-{id}")
                } else {
                    format!("runtime-{id}")
                };
                if save {
                    let session_id = statuses
                        .lock()
                        .unwrap()
                        .get(tab_id.as_str())
                        .map(|s| s.session_id.clone());
                    let Some(mut session) =
                        session_id.and_then(|id| store.borrow().get(&id).cloned())
                    else {
                        return;
                    };
                    session.forwards.push(forward.clone());
                    let mut config = store.borrow_mut();
                    config.upsert(session);
                    if let Err(error) = config.save() {
                        tracing::warn!("failed to save tunnel: {error:#}");
                        return;
                    }
                }
                if let Some(handle) = handles.borrow().get(tab_id.as_str()) {
                    handle.add_tunnel(tunnel_id, forward);
                }
            },
        );
    }
    {
        let handles = handles.clone();
        let store = store.clone();
        let statuses = statuses.clone();
        window.on_tunnel_update(
            move |tab_id, tunnel_id, name, kind, bind, bind_port, host, host_port, save, auto_start| {
                let Some(mut forward) = parse_tunnel_form(
                    &kind, &name, &bind, &bind_port, &host, &host_port, auto_start,
                ) else {
                    return;
                };
                let tunnel_id = tunnel_id.to_string();
                if !handles.borrow().contains_key(tab_id.as_str()) {
                    return;
                }
                if let Some(saved_id) = tunnel_id.strip_prefix("saved-") {
                    forward.id = saved_id.to_string();
                } else if save {
                    if let Some(runtime_id) = tunnel_id.strip_prefix("runtime-") {
                        forward.id = runtime_id.to_string();
                    }
                }
                if !store_edited_tunnel(
                    &store,
                    &statuses,
                    tab_id.as_str(),
                    &tunnel_id,
                    &forward,
                    save,
                ) {
                    return;
                }
                if let Some(handle) = handles.borrow().get(tab_id.as_str()) {
                    handle.update_tunnel(tunnel_id, forward, save);
                }
            },
        );
    }
    {
        let handles = handles.clone();
        window.on_tunnel_stop(move |tab_id, id| {
            if let Some(handle) = handles.borrow().get(tab_id.as_str()) {
                handle.stop_tunnel(id.to_string());
            }
        });
    }
    {
        let handles = handles.clone();
        window.on_tunnel_start(move |tab_id, id| {
            if let Some(handle) = handles.borrow().get(tab_id.as_str()) {
                handle.start_tunnel(id.to_string());
            }
        });
    }
    {
        let weak = window.as_weak();
        let handles = handles.clone();
        window.on_sidebar_tunnel_start(move |id| {
            let Some(win) = weak.upgrade() else { return };
            let tab = win.get_active_tab_id();
            if let Some(handle) = handles.borrow().get(tab.as_str()) {
                handle.start_tunnel(id.to_string());
            }
        });
    }
    {
        let weak = window.as_weak();
        let handles = handles.clone();
        window.on_sidebar_tunnel_stop(move |id| {
            let Some(win) = weak.upgrade() else { return };
            let tab = win.get_active_tab_id();
            if let Some(handle) = handles.borrow().get(tab.as_str()) {
                handle.stop_tunnel(id.to_string());
            }
        });
    }
    {
        let weak = window.as_weak();
        window.on_sidebar_open_tunnels(move || {
            let Some(win) = weak.upgrade() else { return };
            win.set_open_tunnels_seq(win.get_open_tunnels_seq().wrapping_add(1));
        });
    }
    window.on_tunnel_delete(move |tab_id, id| {
        if !handles.borrow().contains_key(tab_id.as_str()) {
            return;
        }
        if let Some(forward_id) = id.strip_prefix("saved-") {
            let session_id = statuses
                .lock()
                .unwrap()
                .get(tab_id.as_str())
                .map(|s| s.session_id.clone());
            if let Some(mut session) = session_id.and_then(|id| store.borrow().get(&id).cloned()) {
                session.forwards.retain(|f| f.id != forward_id);
                let mut config = store.borrow_mut();
                config.upsert(session);
                if let Err(error) = config.save() {
                    tracing::warn!("failed to delete saved tunnel: {error:#}");
                    return;
                }
            }
        }
        if let Some(handle) = handles.borrow().get(tab_id.as_str()) {
            handle.delete_tunnel(id.to_string());
        }
    });
    {
        let weak = window.as_weak();
        window.on_open_tunnel_client(move |tool, bind| {
            let Some(win) = weak.upgrade() else { return };
            match open_tunnel_client(tool.as_str(), bind.as_str()) {
                Ok(status) => {
                    win.set_tunnel_client_hint_error(false);
                    win.set_tunnel_client_hint(status.into());
                }
                Err(err) => {
                    tracing::warn!("tunnel client: {err}");
                    win.set_tunnel_client_hint_error(true);
                    win.set_tunnel_client_hint(err.into());
                }
            }
        });
    }
}

fn parse_tunnel_form(
    kind: &str,
    name: &str,
    bind: &str,
    bind_port: &str,
    host: &str,
    host_port: &str,
    auto_start: bool,
) -> Option<crate::config::PortForward> {
    let kind = kind.to_string();
    if !matches!(kind.as_str(), "local" | "remote" | "dynamic") {
        return None;
    }
    let bind_port = bind_port.trim().parse::<u16>().ok().filter(|port| *port > 0)?;
    let host_port = if kind == "dynamic" {
        0
    } else {
        let port = host_port.trim().parse::<u16>().ok().filter(|port| *port > 0)?;
        if host.trim().is_empty() {
            return None;
        }
        port
    };
    Some(crate::config::PortForward {
        id: String::new(),
        kind,
        auto_start,
        name: name.trim().to_string(),
        bind_addr: bind.trim().to_string(),
        bind_port,
        host: host.trim().to_string(),
        host_port,
    })
}

/// Write the edited rule back onto the SSH session. Runtime-only rules stay
/// unsaved unless the box is checked. Returns false when a save was requested
/// but the session could not be stored.
fn store_edited_tunnel(
    store: &Rc<RefCell<ConfigStore>>,
    statuses: &TabStatuses,
    tab_id: &str,
    tunnel_id: &str,
    forward: &crate::config::PortForward,
    save: bool,
) -> bool {
    let session_id = statuses
        .lock()
        .unwrap()
        .get(tab_id)
        .map(|status| status.session_id.clone());
    let Some(mut session) = session_id.and_then(|id| store.borrow().get(&id).cloned()) else {
        return !save;
    };
    let changed = if let Some(saved_id) = tunnel_id.strip_prefix("saved-") {
        if save {
            if let Some(slot) = session.forwards.iter_mut().find(|rule| rule.id == saved_id) {
                *slot = forward.clone();
            } else {
                session.forwards.push(forward.clone());
            }
        } else {
            session.forwards.retain(|rule| rule.id != saved_id);
        }
        true
    } else if let Some(index) = tunnel_id
        .strip_prefix("config-")
        .and_then(|index| index.parse::<usize>().ok())
    {
        if save {
            if let Some(slot) = session.forwards.get_mut(index) {
                let id = if slot.id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    slot.id.clone()
                };
                *slot = forward.clone();
                slot.id = id;
            }
        } else if index < session.forwards.len() {
            session.forwards.remove(index);
        }
        true
    } else if save {
        let mut stored = forward.clone();
        if stored.id.is_empty() {
            stored.id = uuid::Uuid::new_v4().to_string();
        }
        if let Some(slot) = session.forwards.iter_mut().find(|rule| rule.id == stored.id) {
            *slot = stored;
        } else {
            session.forwards.push(stored);
        }
        true
    } else {
        false
    };
    if !changed {
        return true;
    }
    let mut config = store.borrow_mut();
    config.upsert(session);
    if let Err(error) = config.save() {
        tracing::warn!("failed to update tunnel: {error:#}");
        return false;
    }
    true
}

/// Local address a desktop client should dial for an `-L` rule.
/// `0.0.0.0`, `*` and `::` listen on every interface, so the client uses loopback.
fn local_listen_endpoint(bind: &str) -> Option<(String, u16)> {
    let bind = bind.trim();
    let (host, port) = bind.rsplit_once(':')?;
    let port = port.parse::<u16>().ok().filter(|port| *port > 0)?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    let host = if host.is_empty() || host == "0.0.0.0" || host == "*" || host == "::" {
        "127.0.0.1"
    } else {
        host
    };
    Some((host.to_string(), port))
}

fn open_tunnel_client(tool: &str, bind: &str) -> Result<String, String> {
    let (host, port) = local_listen_endpoint(bind).ok_or_else(|| {
        t(
            "这条规则没有本机监听端口",
            "this rule has no local listen port",
        )
        .to_string()
    })?;
    match tool {
        "rdp" => open_rdp(&host, port).map(|_| String::new()),
        "vnc" => open_vnc(&host, port),
        "browser" => open_browser(&host, port).map(|_| String::new()),
        _ => Err(t("未知的打开方式", "unknown opener").to_string()),
    }
}

fn open_rdp(host: &str, port: u16) -> Result<String, String> {
    crate::rdp::launch_resizable(host, port)
}

fn open_vnc(host: &str, port: u16) -> Result<String, String> {
    let addr = format!("{host}::{port}");
    #[cfg(target_os = "macos")]
    {
        let url = format!("vnc://{host}:{port}");
        if std::process::Command::new("open").arg(&url).spawn().is_ok() {
            return Ok(url);
        }
    }
    let mut commands: Vec<(String, Vec<String>)> = ["vncviewer", "tvnviewer", "xtigervncviewer"]
        .into_iter()
        .map(|name| (name.to_string(), vec![addr.clone()]))
        .collect();
    #[cfg(windows)]
    {
        for key in ["ProgramFiles", "ProgramFiles(x86)"] {
            let Ok(root) = std::env::var(key) else {
                continue;
            };
            for rel in [
                r"TightVNC\tvnviewer.exe",
                r"RealVNC\VNC Viewer\vncviewer.exe",
                r"TigerVNC\vncviewer.exe",
                r"uvnc bvba\UltraVNC\vncviewer.exe",
            ] {
                let path = std::path::Path::new(&root).join(rel);
                if path.is_file() {
                    commands.push((path.display().to_string(), vec![addr.clone()]));
                }
            }
        }
    }
    commands.push((
        "remmina".to_string(),
        vec!["-c".into(), format!("vnc://{host}:{port}")],
    ));
    match spawn_first(&commands) {
        Ok(()) => Ok(format!(
            "{} {host}:{port}{}",
            t("VNC 已启动，正在连接", "VNC started, connecting to"),
            t(
                "。网络较慢时，窗口会等连上后再出现。",
                ". On a slow link the window appears after it connects."
            )
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(t(
            "未找到本机 VNC 客户端。请安装 RealVNC、TightVNC 或 TigerVNC。",
            "No local VNC viewer found. Install RealVNC, TightVNC, or TigerVNC.",
        )
        .to_string()),
        Err(err) => Err(format!(
            "{}: {err}",
            t("无法启动 VNC", "cannot start VNC")
        )),
    }
}

fn open_browser(host: &str, port: u16) -> Result<String, String> {
    let url = format!("http://{host}:{port}");
    let spawned = {
        #[cfg(windows)]
        {
            std::process::Command::new("explorer").arg(&url).spawn()
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open").arg(&url).spawn()
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            std::process::Command::new("xdg-open").arg(&url).spawn()
        }
    };
    spawned
        .map(|_| url)
        .map_err(|err| format!("{}: {err}", t("无法打开浏览器", "cannot open a browser")))
}

fn spawn_first(commands: &[(String, Vec<String>)]) -> std::io::Result<()> {
    let mut last = std::io::Error::new(std::io::ErrorKind::NotFound, "no viewer");
    for (program, args) in commands {
        match std::process::Command::new(program).args(args).spawn() {
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => last = err,
            Err(err) => return Err(err),
        }
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::local_listen_endpoint;

    #[test]
    fn listen_endpoint_uses_loopback_for_wildcard_binds() {
        assert_eq!(
            local_listen_endpoint("0.0.0.0:5900"),
            Some(("127.0.0.1".to_string(), 5900))
        );
        assert_eq!(
            local_listen_endpoint("[::]:3389"),
            Some(("127.0.0.1".to_string(), 3389))
        );
        assert_eq!(
            local_listen_endpoint("127.0.0.1:5901"),
            Some(("127.0.0.1".to_string(), 5901))
        );
        assert_eq!(local_listen_endpoint("socks"), None);
    }
}
