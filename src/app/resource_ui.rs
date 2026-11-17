use super::*;

pub(super) fn push_ring(buf: &mut Vec<f32>, val: f32) {
    if buf.len() != NET_HISTORY_LEN {
        *buf = vec![0.0; NET_HISTORY_LEN];
    }
    buf.remove(0);
    buf.push(val);
}

/// Auto-scale a raw bytes/sec history to 0..1 against its own window peak so the
/// sparkline always uses the full height (like FinalShell's relative graph).
pub(super) fn normalized_model(buf: &[f32]) -> ModelRc<f32> {
    let max = buf.iter().cloned().fold(1.0_f32, f32::max);
    let scaled: Vec<f32> = buf.iter().map(|v| (v / max).clamp(0.0, 1.0)).collect();
    ModelRc::from(Rc::new(VecModel::from(scaled)))
}

/// Build the filesystem-usage model (path, "avail/total", used fraction).
pub(super) fn disk_rows(disks: &[(String, u64, u64)]) -> Vec<DiskInfo> {
    disks
        .iter()
        .map(|(mount, avail, total)| {
            let used = total.saturating_sub(*avail);
            let percent = if *total > 0 {
                used as f32 / *total as f32
            } else {
                0.0
            };
            DiskInfo {
                path: mount.clone().into(),
                detail: format!("{}/{}", format_size(*avail), format_size(*total)).into(),
                percent,
            }
        })
        .collect()
}

pub(super) fn disk_model(disks: &[(String, u64, u64)]) -> ModelRc<DiskInfo> {
    ModelRc::from(Rc::new(VecModel::from(disk_rows(disks))))
}

/// Build the process-monitor model for the popup (#23). `cpu`/`mem` are
/// pre-formatted to one decimal; `cpu_frac` (0..1) drives the row's load bar.
pub(super) fn set_process_action_error(weak: &slint::Weak<ProcWindow>, message: &str) {
    if let Some(window) = weak.upgrade() {
        window.set_action_busy(false);
        window.set_action_error(true);
        window.set_action_status(message.into());
    }
}

/// A root login can signal any process directly. Non-root logins may signal
/// only their own processes; root and other users' processes require `su`.
pub(super) fn process_needs_root(current_user: &str, process_user: &str) -> bool {
    current_user != "root" && process_user != current_user
}

pub(super) fn proc_rows(procs: &[ProcInfo], current_user: &str, tab_id: &str) -> Vec<ProcRow> {
    procs
        .iter()
        .map(|p| ProcRow {
            tab_id: tab_id.into(),
            pid: p.pid.to_string().into(),
            user: p.user.clone().into(),
            cpu: format!("{:.1}", p.cpu).into(),
            mem: format!("{:.1}", p.mem).into(),
            command: p.command.clone().into(),
            cpu_frac: (p.cpu / 100.0).clamp(0.0, 1.0),
            mem_frac: (p.mem / 100.0).clamp(0.0, 1.0),
            own_process: !process_needs_root(current_user, &p.user),
        })
        .collect()
}

/// Rows for the process window, ordered by the column the user picked.
pub(super) fn ordered_proc_rows(
    procs: &[ProcInfo],
    current_user: &str,
    tab_id: &str,
    sort_key: &str,
    sort_dir: i32,
) -> Vec<ProcRow> {
    let mut items = procs.to_vec();
    sort_procs(&mut items, sort_key, sort_dir);
    proc_rows(&items, current_user, tab_id)
}

fn sort_procs(procs: &mut [ProcInfo], sort_key: &str, sort_dir: i32) {
    procs.sort_by(|a, b| {
        let ord = match sort_key {
            "pid" => a.pid.cmp(&b.pid),
            "user" => a
                .user
                .to_ascii_lowercase()
                .cmp(&b.user.to_ascii_lowercase())
                .then(a.pid.cmp(&b.pid)),
            "mem" => a.mem.total_cmp(&b.mem).then(a.pid.cmp(&b.pid)),
            "command" => a
                .command
                .to_ascii_lowercase()
                .cmp(&b.command.to_ascii_lowercase())
                .then(a.pid.cmp(&b.pid)),
            _ => a
                .cpu
                .total_cmp(&b.cpu)
                .then(a.mem.total_cmp(&b.mem))
                .then(a.pid.cmp(&b.pid)),
        };
        if sort_dir < 0 { ord.reverse() } else { ord }
    });
}

/// Next column and direction after a header click. A new column starts at a
/// useful direction; the same column flips.
pub(super) fn next_proc_sort(current_key: &str, current_dir: i32, clicked: &str) -> (String, i32) {
    if current_key == clicked {
        (clicked.to_string(), if current_dir < 0 { 1 } else { -1 })
    } else {
        let dir = match clicked {
            "user" | "command" => 1,
            _ => -1,
        };
        (clicked.to_string(), dir)
    }
}

/// Re-order the rows already on screen. Used when refresh is paused.
pub(super) fn resort_process_model(win: &AppWindow) {
    let key = win.get_proc_sort_key().to_string();
    let dir = win.get_proc_sort_dir();
    let list = win.get_proc_list();
    let Some(model) = list.as_any().downcast_ref::<VecModel<ProcRow>>() else {
        return;
    };
    let mut rows = Vec::with_capacity(model.row_count());
    for i in 0..model.row_count() {
        if let Some(row) = model.row_data(i) {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| proc_row_order(a, b, &key, dir));
    model.set_vec(rows);
}

fn proc_text_num(text: &str) -> f32 {
    text.parse().unwrap_or(0.0)
}

fn proc_row_order(a: &ProcRow, b: &ProcRow, sort_key: &str, sort_dir: i32) -> std::cmp::Ordering {
    let pid = |row: &ProcRow| row.pid.parse::<u32>().unwrap_or(0);
    let ord = match sort_key {
        "pid" => pid(a).cmp(&pid(b)),
        "user" => a
            .user
            .to_ascii_lowercase()
            .cmp(&b.user.to_ascii_lowercase())
            .then(pid(a).cmp(&pid(b))),
        "mem" => proc_text_num(&a.mem)
            .total_cmp(&proc_text_num(&b.mem))
            .then(pid(a).cmp(&pid(b))),
        "command" => a
            .command
            .to_ascii_lowercase()
            .cmp(&b.command.to_ascii_lowercase())
            .then(pid(a).cmp(&pid(b))),
        _ => proc_text_num(&a.cpu)
            .total_cmp(&proc_text_num(&b.cpu))
            .then(proc_text_num(&a.mem).total_cmp(&proc_text_num(&b.mem)))
            .then(pid(a).cmp(&pid(b))),
    };
    if sort_dir < 0 { ord.reverse() } else { ord }
}

#[cfg(test)]
#[path = "../../tests/app/process_monitor/mod.rs"]
mod process_row_tests;

pub(super) fn metric_rows(
    cpu: f32,
    mem: f32,
    swap: f32,
    mem_detail: impl Into<SharedString>,
    swap_detail: impl Into<SharedString>,
) -> Vec<SysMetricRow> {
    vec![
        SysMetricRow {
            label: "CPU".into(),
            percent: cpu,
            detail: "".into(),
            kind: 0,
        },
        SysMetricRow {
            label: t("内存", "Memory").into(),
            percent: mem,
            detail: mem_detail.into(),
            kind: 1,
        },
        SysMetricRow {
            label: t("交换", "Swap").into(),
            percent: swap,
            detail: swap_detail.into(),
            kind: 2,
        },
    ]
}

pub(super) fn net_rows(net: &[(String, u64, u64)]) -> Vec<SysNetRow> {
    net.iter()
        .map(|(name, rx, tx)| SysNetRow {
            name: name.clone().into(),
            up: format_bytes_per_sec(*tx).into(),
            down: format_bytes_per_sec(*rx).into(),
        })
        .collect()
}

pub(super) fn pairs_to_overview_rows(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    pairs
        .chunks(2)
        .map(|chunk| {
            let first = &chunk[0];
            let second = chunk.get(1);
            SysInfoRow {
                c1: first.0.clone().into(),
                c2: first.1.clone().into(),
                c3: second.map(|p| p.0.clone()).unwrap_or_default().into(),
                c4: second.map(|p| p.1.clone()).unwrap_or_default().into(),
                c5: "".into(),
            }
        })
        .collect()
}

pub(super) fn pairs_to_one_row(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    let value = |idx: usize| {
        pairs
            .get(idx)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "-".to_string())
    };
    vec![SysInfoRow {
        c1: value(0).into(),
        c2: value(1).into(),
        c3: value(2).into(),
        c4: value(3).into(),
        c5: value(4).into(),
    }]
}

pub(super) fn pairs_to_rows(pairs: &[(String, String)], width: usize) -> Vec<SysInfoRow> {
    pairs
        .chunks(width)
        .filter(|chunk| {
            chunk
                .iter()
                .any(|(_, v)| !v.trim().is_empty() && v.trim() != "-")
        })
        .map(|chunk| {
            let value = |idx: usize| {
                chunk
                    .get(idx)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| "-".to_string())
            };
            SysInfoRow {
                c1: value(0).into(),
                c2: value(1).into(),
                c3: value(2).into(),
                c4: value(3).into(),
                c5: value(4).into(),
            }
        })
        .collect()
}

pub(super) fn cpu_usage_detail_rows(pairs: &[(String, String)]) -> Vec<SysInfoRow> {
    let value = |idx: usize| {
        pairs
            .get(idx)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "0.0%".to_string())
    };
    let extra = pairs
        .iter()
        .skip(4)
        .map(|(k, v)| format!("{k} {v}"))
        .collect::<Vec<_>>()
        .join(" / ");
    vec![SysInfoRow {
        c1: value(0).into(),
        c2: value(2).into(),
        c3: value(1).into(),
        c4: value(3).into(),
        c5: extra.into(),
    }]
}

pub(super) fn tuple5_rows(rows: &[(String, String, String, String, String)]) -> Vec<SysInfoRow> {
    rows.iter()
        .map(|r| SysInfoRow {
            c1: r.0.clone().into(),
            c2: r.1.clone().into(),
            c3: r.2.clone().into(),
            c4: r.3.clone().into(),
            c5: r.4.clone().into(),
        })
        .collect()
}

/// Mirror the main window's theme/scale/UI-font onto the detached process
/// window. Theme is a per-window Slint global, so a detached window keeps its
/// compile-time (dark) defaults until we copy these across (#23).
pub(super) fn sync_proc_theme(main: &AppWindow, proc: &ProcWindow) {
    proc.set_dark_mode(main.get_dark_mode());
    proc.set_ui_scale(main.get_ui_scale());
    proc.set_ui_font_family(main.get_ui_font_family());
    // Mirror the immersive wallpaper so the detached window shares the frosted
    // backdrop instead of a flat panel.
    proc.set_wallpaper_img(main.get_wallpaper_img());
    proc.set_wallpaper_active(main.get_wallpaper_active());
    proc.set_wp_accent(main.get_wp_accent());
    proc.set_wp_tint(main.get_wp_tint());
}

pub(super) fn sync_system_info_theme(main: &AppWindow, sys: &SystemInfoWindow) {
    sys.set_dark_mode(main.get_dark_mode());
    sys.set_ui_scale(main.get_ui_scale());
    sys.set_ui_font_family(main.get_ui_font_family());
    sys.set_wallpaper_img(main.get_wallpaper_img());
    sys.set_wallpaper_active(main.get_wallpaper_active());
    sys.set_wp_accent(main.get_wp_accent());
    sys.set_wp_tint(main.get_wp_tint());
}

pub(super) fn sync_editor_theme(main: &AppWindow, editor: &EditorWindow) {
    editor.set_dark_mode(main.get_dark_mode());
    editor.set_ui_scale(main.get_ui_scale());
    editor.set_ui_font_family(main.get_ui_font_family());
    editor.set_wallpaper_img(main.get_wallpaper_img());
    editor.set_wallpaper_active(main.get_wallpaper_active());
    editor.set_wp_accent(main.get_wp_accent());
    editor.set_wp_tint(main.get_wp_tint());
}

pub(super) fn place_system_info_window(main: &AppWindow, sys: &SystemInfoWindow) {
    let Some((mon_x, mon_y, mon_w, mon_h)) = main
        .window()
        .with_winit_window(|ww| {
            let scale = ww.scale_factor().max(0.01);
            let monitor = ww.current_monitor().or_else(|| ww.primary_monitor())?;
            let pos = monitor.position();
            let size = monitor.size();
            Some((
                pos.x as f64 / scale,
                pos.y as f64 / scale,
                size.width as f64 / scale,
                size.height as f64 / scale,
            ))
        })
        .flatten()
    else {
        return;
    };

    let target_w = (mon_w * 0.5).clamp(760.0, (mon_w - 24.0).max(760.0));
    let target_h = (mon_h * 0.5).clamp(520.0, (mon_h - 24.0).max(520.0));
    let x = mon_x + (mon_w - target_w).max(0.0) / 2.0;
    let y = mon_y + (mon_h - target_h).max(0.0) / 2.0;

    // Use the Slint window API instead of the winit handle: hidden windows are
    // no longer materialized eagerly (they map on Wayland and pollute the
    // taskbar — see the vendor patch in i-slint-backend-winit), so the first
    // open may run before the native window exists. In that state the Slint
    // API stores the size/position on the adapter and applies it at creation;
    // with a live window it behaves like the winit calls did.
    sys.window()
        .set_size(slint::LogicalSize::new(target_w as f32, target_h as f32));
    sys.window()
        .set_position(slint::LogicalPosition::new(x as f32, y as f32));
}

/// Center the process monitor on the same physical monitor as the main window.
/// A window that is still at the old narrow size is widened so the command
/// column can show a full line. A window the user has already made wider is
/// left at that size.
pub(super) fn place_process_window(main: &AppWindow, process: &ProcWindow) {
    let Some((mon_x, mon_y, mon_w, mon_h)) = main
        .window()
        .with_winit_window(|ww| {
            let scale = ww.scale_factor().max(0.01);
            let monitor = ww.current_monitor().or_else(|| ww.primary_monitor())?;
            let pos = monitor.position();
            let size = monitor.size();
            Some((
                pos.x as f64 / scale,
                pos.y as f64 / scale,
                size.width as f64 / scale,
                size.height as f64 / scale,
            ))
        })
        .flatten()
    else {
        return;
    };

    let current = process.window().size();
    let wide_enough = current.width >= 1000 && current.height >= 520;
    let target_w = if wide_enough {
        current.width as f64
    } else {
        (mon_w * 0.62).clamp(1000.0, (mon_w - 48.0).max(1000.0))
    };
    let target_h = if wide_enough {
        current.height as f64
    } else {
        (mon_h * 0.62).clamp(560.0, (mon_h - 48.0).max(560.0))
    };
    let x = mon_x + (mon_w - target_w).max(0.0) / 2.0;
    let y = mon_y + (mon_h - target_h).max(0.0) / 2.0;
    if !wide_enough {
        process
            .window()
            .set_size(slint::LogicalSize::new(target_w as f32, target_h as f32));
    }
    process
        .window()
        .set_position(slint::LogicalPosition::new(x as f32, y as f32));
}
