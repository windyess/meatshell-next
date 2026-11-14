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
                let kind = kind.to_string();
                if !matches!(kind.as_str(), "local" | "remote" | "dynamic") {
                    return;
                }
                let Ok(bind_port) = bind_port
                    .trim()
                    .parse::<u16>()
                    .ok()
                    .filter(|p| *p > 0)
                    .ok_or(())
                else {
                    return;
                };
                let host_port = if kind == "dynamic" {
                    0
                } else {
                    match host_port.trim().parse::<u16>().ok().filter(|p| *p > 0) {
                        Some(port) if !host.trim().is_empty() => port,
                        _ => return,
                    }
                };
                if !handles.borrow().contains_key(tab_id.as_str()) {
                    return;
                }
                let id = uuid::Uuid::new_v4().to_string();
                let forward = crate::config::PortForward {
                    id: if save { id.clone() } else { String::new() },
                    kind,
                    auto_start,
                    name: name.trim().to_string(),
                    bind_addr: bind.trim().to_string(),
                    bind_port,
                    host: host.trim().to_string(),
                    host_port,
                };
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
}
