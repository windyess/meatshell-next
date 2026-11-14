use super::*;

/// Bundled icon key → `assets/file-icons/{shell|fluent|system}/`.
/// Priority: Windows Explorer (shell) → Fluent File Type → System.
pub(super) fn sftp_entry_icon_kind(
    name: &str,
    is_dir: bool,
    file_type: &str,
    mode: u32,
) -> SharedString {
    if is_dir || file_type == "directory" {
        return "folder".into();
    }
    if file_type == "symlink" {
        return "link".into();
    }

    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let kind = match ext.as_str() {
        // shell/ — Windows Explorer associations (preferred look)
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "heic" | "avif" | "tif"
        | "tiff" | "psd" => "image",
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v" | "flv" | "wmv" | "mpeg" | "mpg" => {
            "video"
        }
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" | "opus" => "audio",
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "iso" | "dmg" => {
            "archive"
        }
        "pdf" => "pdf",
        "xls" | "xlsx" | "xlsm" | "xlsb" | "ods" => "table",
        "csv" => "csv",
        "doc" | "docx" | "docm" | "odt" => "document",
        "rtf" | "pages" | "epub" => "rtf",
        "ppt" | "pptx" | "pptm" | "odp" | "key" => "presentation",
        "html" | "htm" | "mht" | "mhtml" => "html",
        "xml" | "xsl" | "xaml" => "xml",
        "json" | "yml" | "yaml" | "ini" | "conf" | "cfg" | "plist" => "config",
        "txt" | "log" | "rst" | "org" => "text",
        "py" | "js" | "jsx" | "ts" | "tsx" | "c" | "h" | "cpp" | "hpp" | "java" | "kt"
        | "swift" | "rb" | "php" | "lua" | "cs" | "css" | "scss" | "sql" => "code",
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => "terminal",
        "exe" | "msi" | "apk" | "app" | "appimage" | "deb" | "rpm" => "exe",
        "dll" | "so" | "dylib" | "jar" | "pdb" => "dll",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "eml" | "msg" | "pst" | "ost" => "email",
        "md" | "markdown" | "mdown" | "mkdn" => "md",
        "toml" => "toml",
        "rs" => "rs",
        "go" => "go",
        "vue" => "vue",
        "svelte" => "svelte",
        "wasm" => "wasm",
        "env" => "env",
        "gitignore" => "gitignore",

        // fluent/ — types Shell has no distinct association for
        "svg" | "svgz" | "eps" | "ai" | "sketch" => "vector",
        "vsd" | "vsdx" | "vsdm" => "vsdx",
        "one" | "onetoc" | "onetoc2" | "onepkg" => "one",
        "pbix" | "pbids" => "powerbi",
        "ipynb" => "ipynb",
        "accdb" | "mdb" => "accdb",
        "mpp" => "mpp",
        "pub" => "pub",
        "obj" | "fbx" | "gltf" | "glb" | "stl" | "blend" | "dwg" | "dxf" => "model",
        "ics" | "ical" | "vcs" => "calendar",
        _ if mode & 0o111 != 0 => "terminal",
        _ => "file",
    };
    kind.into()
}

pub(super) fn terminal_sftp_paths(w: &AppWindow) -> HashMap<String, String> {
    use slint::Model as _;
    let mut out = HashMap::new();
    let model = w.get_terminals();
    if let Some(terminals) = model.as_any().downcast_ref::<VecModel<TerminalState>>() {
        for i in 0..terminals.row_count() {
            if let Some(row) = terminals.row_data(i) {
                out.insert(row.id.to_string(), row.sftp_path.to_string());
            }
        }
    }
    out
}

pub(super) fn sorted_sftp_entries_from_model(
    model: &ModelRc<SftpEntry>,
    key: &str,
    dir: i32,
) -> ModelRc<SftpEntry> {
    let Some(vec_model) = model.as_any().downcast_ref::<VecModel<SftpEntry>>() else {
        return model.clone();
    };
    let mut entries = Vec::with_capacity(vec_model.row_count());
    for i in 0..vec_model.row_count() {
        if let Some(entry) = vec_model.row_data(i) {
            entries.push(entry);
        }
    }
    sort_sftp_entries(&mut entries, key, dir);
    ModelRc::from(std::rc::Rc::new(VecModel::from(entries)))
}

/// Default file list: modification time, newest on the first row.
pub(super) const SFTP_DEFAULT_SORT_KEY: &str = "modified";
pub(super) const SFTP_DEFAULT_SORT_DIR: i32 = -1;

pub(super) fn sort_sftp_entries(entries: &mut [SftpEntry], key: &str, dir: i32) {
    use std::cmp::Ordering;

    let (key, dir) = if dir == 0 || key.is_empty() {
        (SFTP_DEFAULT_SORT_KEY, SFTP_DEFAULT_SORT_DIR)
    } else {
        (key, dir)
    };
    // Lowercase once. The comparator used to allocate two strings per comparison.
    let names: Vec<String> = entries.iter().map(|e| e.name.to_lowercase()).collect();
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by(|&ia, &ib| {
        let a = &entries[ia];
        let b = &entries[ib];
        // Time order mixes files and folders so the newest item is row 0.
        // Other columns still keep directories together.
        let group = if key == "modified" {
            Ordering::Equal
        } else {
            match (a.is_dir, b.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        };
        if group != Ordering::Equal {
            return group;
        }
        let name_ord = natural_ascii_cmp(&names[ia], &names[ib]).then_with(|| a.name.cmp(&b.name));
        let ord = match key {
            "size" => a
                .size_bytes
                .partial_cmp(&b.size_bytes)
                .unwrap_or(Ordering::Equal)
                .then_with(|| name_ord),
            "modified" => a.modified_ts.cmp(&b.modified_ts).then_with(|| name_ord),
            "permissions" => a
                .permissions_mode
                .cmp(&b.permissions_mode)
                .then_with(|| name_ord),
            "owner" => a.owner.cmp(&b.owner).then_with(|| name_ord),
            "group" => a.group.cmp(&b.group).then_with(|| name_ord),
            "type" => a.file_type.cmp(&b.file_type).then_with(|| name_ord),
            _ => name_ord,
        };
        if dir < 0 { ord.reverse() } else { ord }
    });
    let sorted: Vec<SftpEntry> = order.iter().map(|&i| entries[i].clone()).collect();
    entries.clone_from_slice(&sorted);
}

/// Format and sort a remote listing. Safe to call off the UI thread.
pub(super) fn sftp_entries_from_remote(entries: &[crate::ssh::RemoteEntry]) -> Vec<SftpEntry> {
    entries
        .iter()
        .map(|e| {
            let icon_kind = sftp_entry_icon_kind(&e.name, e.is_dir, &e.file_type, e.mode);
            SftpEntry {
                name: e.name.clone().into(),
                full_path: e.full_path.clone().into(),
                is_dir: e.is_dir,
                is_link: e.file_type == "symlink",
                size: if e.is_dir {
                    "".into()
                } else {
                    format_size(e.size).into()
                },
                size_bytes: e.size as f32,
                modified: format_mtime(e.modified).into(),
                modified_ts: e.modified as i32,
                mode: (e.mode & 0o7777) as i32,
                file_type: match e.file_type.as_str() {
                    "directory" => crate::i18n::t("文件夹", "Folder"),
                    "symlink" => crate::i18n::t("符号链接", "Symbolic link"),
                    "socket" => crate::i18n::t("套接字", "Socket"),
                    "block-device" => crate::i18n::t("块设备", "Block device"),
                    "character-device" => crate::i18n::t("字符设备", "Character device"),
                    "fifo" => crate::i18n::t("管道", "FIFO"),
                    _ => crate::i18n::t("文件", "File"),
                }
                .into(),
                permissions: crate::ssh::format_permissions(e.permissions_mode).into(),
                permissions_mode: e.permissions_mode as i32,
                owner: e.owner.clone().unwrap_or_default().into(),
                group: e.group.clone().unwrap_or_default().into(),
                selected: false,
                icon_kind,
            }
        })
        .collect()
}

fn sftp_listing_tickets() -> &'static std::sync::Mutex<HashMap<String, u64>> {
    static TICKETS: std::sync::OnceLock<std::sync::Mutex<HashMap<String, u64>>> =
        std::sync::OnceLock::new();
    TICKETS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// Build the file list off the UI thread, then install it if this request is
/// still the newest one for the tab.
pub(super) fn schedule_sftp_listing(
    win: &AppWindow,
    tab_id: &str,
    path: String,
    entries: Vec<crate::ssh::RemoteEntry>,
    sort_key: String,
    sort_dir: i32,
) {
    let ticket = {
        let mut tickets = sftp_listing_tickets().lock().unwrap();
        let next = tickets.entry(tab_id.to_string()).or_insert(0);
        *next = next.wrapping_add(1);
        *next
    };
    let weak = win.as_weak();
    let tab = tab_id.to_string();
    std::thread::spawn(move || {
        let mut slint_entries = sftp_entries_from_remote(&entries);
        sort_sftp_entries(&mut slint_entries, &sort_key, sort_dir);
        let _ = slint::invoke_from_event_loop(move || {
            let current = sftp_listing_tickets()
                .lock()
                .unwrap()
                .get(&tab)
                .copied();
            if current != Some(ticket) {
                return;
            }
            let Some(win) = weak.upgrade() else { return };
            install_sftp_listing(&win, &tab, &path, slint_entries);
        });
    });
}

pub(super) fn install_sftp_listing(
    win: &AppWindow,
    tab_id: &str,
    path: &str,
    entries: Vec<SftpEntry>,
) {
    let model = ModelRc::from(std::rc::Rc::new(VecModel::from(entries)));
    let path: SharedString = path.into();
    set_terminal_row(win, tab_id, move |t| {
        t.sftp_path = path.clone();
        t.sftp_entries = model.clone();
        t.sftp_loading = false;
        t.sftp_selected_count = 0;
    });
}

pub(super) fn natural_ascii_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut ai = 0;
    let mut bi = 0;
    while ai < ab.len() && bi < bb.len() {
        let ad = ab[ai].is_ascii_digit();
        let bd = bb[bi].is_ascii_digit();
        if ad && bd {
            let a_start = ai;
            let b_start = bi;
            while ai < ab.len() && ab[ai].is_ascii_digit() {
                ai += 1;
            }
            while bi < bb.len() && bb[bi].is_ascii_digit() {
                bi += 1;
            }

            let mut a_sig = a_start;
            let mut b_sig = b_start;
            while a_sig < ai && ab[a_sig] == b'0' {
                a_sig += 1;
            }
            while b_sig < bi && bb[b_sig] == b'0' {
                b_sig += 1;
            }

            let a_len = ai - a_sig;
            let b_len = bi - b_sig;
            let ord = a_len
                .cmp(&b_len)
                .then_with(|| ab[a_sig..ai].cmp(&bb[b_sig..bi]))
                .then_with(|| (ai - a_start).cmp(&(bi - b_start)));
            if ord != Ordering::Equal {
                return ord;
            }
            continue;
        }

        let ord = ab[ai].cmp(&bb[bi]);
        if ord != Ordering::Equal {
            return ord;
        }
        ai += 1;
        bi += 1;
    }
    ab.len().cmp(&bb.len())
}

/// Collect the remote paths currently selected in an SFTP tab's file list.
pub(super) fn collect_sftp_selected(
    terminals: &VecModel<TerminalState>,
    tab_id: &str,
) -> Vec<String> {
    let mut paths = Vec::new();
    for ti in 0..terminals.row_count() {
        let Some(row) = terminals.row_data(ti) else {
            continue;
        };
        if row.id.as_str() != tab_id {
            continue;
        }
        if let Some(em) = row
            .sftp_entries
            .as_any()
            .downcast_ref::<VecModel<SftpEntry>>()
        {
            for ei in 0..em.row_count() {
                if let Some(e) = em.row_data(ei) {
                    if e.selected {
                        paths.push(e.full_path.to_string());
                    }
                }
            }
        }
        break;
    }
    paths
}

/// Explorer-style list selection: click / Ctrl+click / Shift+click.
pub(super) fn apply_list_selection(
    selected: &mut [bool],
    idx: usize,
    anchor: i32,
    ctrl: bool,
    shift: bool,
) {
    let n = selected.len();
    if idx >= n {
        return;
    }
    let range = shift && anchor >= 0 && (anchor as usize) < n;
    if range {
        let a = anchor as usize;
        let lo = a.min(idx);
        let hi = a.max(idx);
        for (i, on) in selected.iter_mut().enumerate() {
            if i >= lo && i <= hi {
                *on = true;
            } else if !ctrl {
                *on = false;
            }
        }
        return;
    }
    if ctrl {
        selected[idx] = !selected[idx];
        return;
    }
    for (i, on) in selected.iter_mut().enumerate() {
        *on = i == idx;
    }
}

/// If `path` is already shown with the same set of entries, patch owner/group
/// in place (by `full_path`, order-independent) and keep selection. Used for
/// the getent follow-up paint so directories avoid a second full model rebuild.
pub(super) fn try_patch_sftp_owners(
    terminals: &VecModel<TerminalState>,
    tab_id: &str,
    path: &str,
    entries: &[crate::ssh::RemoteEntry],
) -> bool {
    use std::collections::HashMap;

    for ti in 0..terminals.row_count() {
        let Some(row) = terminals.row_data(ti) else {
            continue;
        };
        if row.id.as_str() != tab_id {
            continue;
        }
        if row.sftp_path.as_str() != path {
            return false;
        }
        let Some(em) = row
            .sftp_entries
            .as_any()
            .downcast_ref::<VecModel<SftpEntry>>()
        else {
            return false;
        };
        if em.row_count() != entries.len() {
            return false;
        }
        let by_path: HashMap<&str, (&str, &str)> = entries
            .iter()
            .map(|e| {
                (
                    e.full_path.as_str(),
                    (
                        e.owner.as_deref().unwrap_or(""),
                        e.group.as_deref().unwrap_or(""),
                    ),
                )
            })
            .collect();
        for ei in 0..em.row_count() {
            let Some(cur) = em.row_data(ei) else {
                return false;
            };
            if !by_path.contains_key(cur.full_path.as_str()) {
                return false;
            }
        }
        for ei in 0..em.row_count() {
            let Some(mut cur) = em.row_data(ei) else {
                continue;
            };
            let Some(&(owner, group)) = by_path.get(cur.full_path.as_str()) else {
                continue;
            };
            if cur.owner.as_str() == owner && cur.group.as_str() == group {
                continue;
            }
            cur.owner = owner.into();
            cur.group = group.into();
            em.set_row_data(ei, cur);
        }
        return true;
    }
    false
}

/// Show / hide the SFTP panel loading overlay for a tab (navigation / refresh).
pub(super) fn set_sftp_loading(terminals: &VecModel<TerminalState>, tab_id: &str, loading: bool) {
    for ti in 0..terminals.row_count() {
        let Some(row) = terminals.row_data(ti) else {
            continue;
        };
        if row.id.as_str() != tab_id {
            continue;
        }
        if row.sftp_loading == loading {
            break;
        }
        let mut r = row.clone();
        r.sftp_loading = loading;
        terminals.set_row_data(ti, r);
        break;
    }
}

/// Uncheck every SFTP entry for a tab and reset its selected-count (#100).
pub(super) fn clear_sftp_selection(terminals: &VecModel<TerminalState>, tab_id: &str) {
    for ti in 0..terminals.row_count() {
        let Some(row) = terminals.row_data(ti) else {
            continue;
        };
        if row.id.as_str() != tab_id {
            continue;
        }
        if let Some(em) = row
            .sftp_entries
            .as_any()
            .downcast_ref::<VecModel<SftpEntry>>()
        {
            for ei in 0..em.row_count() {
                if let Some(mut e) = em.row_data(ei) {
                    if e.selected {
                        e.selected = false;
                        em.set_row_data(ei, e);
                    }
                }
            }
        }
        let mut r = row.clone();
        r.sftp_selected_count = 0;
        terminals.set_row_data(ti, r);
        break;
    }
}

pub(super) fn posix_shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn file_name(path: &str) -> &str {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

/// Paths to copy: multi-select wins; otherwise the right-clicked item.
pub(super) fn collect_copy_sources(
    terminals: &VecModel<TerminalState>,
    tab_id: &str,
    menu_target: &str,
) -> Vec<String> {
    let selected = collect_sftp_selected(terminals, tab_id);
    if selected.len() >= 2 {
        selected
    } else if !menu_target.is_empty() {
        vec![menu_target.to_string()]
    } else {
        selected
    }
}

/// Build a POSIX `mv` command. Same-directory sources are skipped (already
/// at the destination); if nothing remains to move, returns `None`.
pub(super) fn build_remote_move_command(sources: &[String], dest_dir: &str) -> Option<String> {
    if sources.is_empty() {
        return None;
    }
    let dest = dest_dir.trim_end_matches('/');
    let dest = if dest.is_empty() { "/" } else { dest };
    let mut elsewhere = Vec::new();
    for src in sources {
        if src.is_empty() {
            continue;
        }
        if parent_path(src) != dest {
            elsewhere.push(src.as_str());
        }
    }
    if elsewhere.is_empty() {
        return None;
    }
    let mut cmd = String::from("mv --");
    for src in &elsewhere {
        cmd.push(' ');
        cmd.push_str(&posix_shell_quote(src));
    }
    let dest_arg = if dest == "/" {
        "/".to_string()
    } else {
        format!("{dest}/")
    };
    cmd.push(' ');
    cmd.push_str(&posix_shell_quote(&dest_arg));
    Some(cmd)
}

/// Build a POSIX `cp -a` command. Same-directory paste uses `name.copy`
/// so GNU cp does not fail with “are the same file”.
pub(super) fn build_remote_copy_command(sources: &[String], dest_dir: &str) -> Option<String> {
    if sources.is_empty() {
        return None;
    }
    let dest = dest_dir.trim_end_matches('/');
    let dest = if dest.is_empty() { "/" } else { dest };
    let mut same_dir = Vec::new();
    let mut elsewhere = Vec::new();
    for src in sources {
        if src.is_empty() {
            continue;
        }
        if parent_path(src) == dest {
            let name = file_name(src);
            let dest_path = if dest == "/" {
                format!("/{name}.copy")
            } else {
                format!("{dest}/{name}.copy")
            };
            same_dir.push(format!(
                "cp -a -- {} {}",
                posix_shell_quote(src),
                posix_shell_quote(&dest_path)
            ));
        } else {
            elsewhere.push(src.as_str());
        }
    }
    let mut parts = Vec::new();
    if !elsewhere.is_empty() {
        let mut cmd = String::from("cp -a --");
        for src in &elsewhere {
            cmd.push(' ');
            cmd.push_str(&posix_shell_quote(src));
        }
        let dest_arg = if dest == "/" {
            "/".to_string()
        } else {
            format!("{dest}/")
        };
        cmd.push(' ');
        cmd.push_str(&posix_shell_quote(&dest_arg));
        parts.push(cmd);
    }
    parts.extend(same_dir);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" && "))
    }
}

/// Build a remote `zip -rq` that archives files/dirs in the current listing.
/// Only sources whose parent is `dest_dir` are included (the visible folder).
/// Existing `name.zip` is renamed to `name-N.zip` so the command does not overwrite.
pub(super) fn build_remote_zip_command(sources: &[String], dest_dir: &str) -> Option<String> {
    let dest = dest_dir.trim_end_matches('/');
    let dest = if dest.is_empty() { "/" } else { dest };
    let names: Vec<&str> = sources
        .iter()
        .filter(|src| !src.is_empty() && parent_path(src) == dest)
        .map(|src| file_name(src))
        .filter(|name| *name != "." && *name != "..")
        .collect();
    if names.is_empty() {
        return None;
    }
    let dest_q = posix_shell_quote(dest);
    let base_q = posix_shell_quote(names[0]);
    let mut cmd = format!(
        "cd -- {dest_q} && n=1 && base={base_q} && out=\"$base.zip\" && \
while [ -e \"$out\" ]; do n=$((n+1)); out=\"$base-$n.zip\"; done && zip -rq \"$out\""
    );
    for name in names {
        cmd.push(' ');
        cmd.push_str(&posix_shell_quote(name));
    }
    Some(cmd)
}

/// Remote extract command for one archive in the current directory.
/// The tool is chosen from the filename. Disk images return `None`.
pub(super) fn build_remote_extract_command(path: &str, dest_dir: &str) -> Option<String> {
    let dest = dest_dir.trim_end_matches('/');
    let dest = if dest.is_empty() { "/" } else { dest };
    if path.is_empty() || parent_path(path) != dest {
        return None;
    }
    let name = file_name(path);
    if name == "." || name == ".." {
        return None;
    }
    let tool = extract_tool(name)?;
    Some(format!(
        "cd -- {} && {tool} {}",
        posix_shell_quote(dest),
        posix_shell_quote(name)
    ))
}

fn extract_tool(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Some("tar -xzf")
    } else if lower.ends_with(".tar.bz2") || lower.ends_with(".tbz2") {
        Some("tar -xjf")
    } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        Some("tar -xJf")
    } else if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        Some("tar --zstd -xf")
    } else if lower.ends_with(".tar") {
        Some("tar -xf")
    } else if lower.ends_with(".zip") {
        Some("unzip -o")
    } else if lower.ends_with(".7z") {
        Some("7z x -y")
    } else if lower.ends_with(".rar") {
        Some("unrar x -o+")
    } else if lower.ends_with(".gz") {
        Some("gzip -dk")
    } else if lower.ends_with(".bz2") {
        Some("bzip2 -dk")
    } else if lower.ends_with(".xz") {
        Some("xz -dk")
    } else if lower.ends_with(".zst") {
        Some("zstd -d -k")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_selects_only_that_row() {
        let mut s = vec![true, false, true];
        apply_list_selection(&mut s, 1, 0, false, false);
        assert_eq!(s, vec![false, true, false]);
    }

    #[test]
    fn ctrl_click_toggles() {
        let mut s = vec![true, false, false];
        apply_list_selection(&mut s, 1, 0, true, false);
        assert_eq!(s, vec![true, true, false]);
        apply_list_selection(&mut s, 0, 1, true, false);
        assert_eq!(s, vec![false, true, false]);
    }

    #[test]
    fn shift_click_selects_range_from_anchor() {
        let mut s = vec![false; 5];
        apply_list_selection(&mut s, 3, 1, false, true);
        assert_eq!(s, vec![false, true, true, true, false]);
    }

    #[test]
    fn ctrl_shift_adds_range() {
        let mut s = vec![true, false, false, false, true];
        apply_list_selection(&mut s, 2, 1, true, true);
        assert_eq!(s, vec![true, true, true, false, true]);
    }

    #[test]
    fn quotes_spaces_and_quotes() {
        assert_eq!(posix_shell_quote("a b"), "'a b'");
        assert_eq!(posix_shell_quote("a'b"), "'a'\\''b'");
        assert_eq!(posix_shell_quote(""), "''");
    }

    #[test]
    fn copies_into_another_directory() {
        let cmd = build_remote_copy_command(
            &["/data/tools/sysctl.conf".into(), "/data/tools/fw-all.sh".into()],
            "/tmp",
        )
        .unwrap();
        assert_eq!(
            cmd,
            "cp -a -- '/data/tools/sysctl.conf' '/data/tools/fw-all.sh' '/tmp/'"
        );
    }

    #[test]
    fn moves_into_another_directory() {
        let cmd = build_remote_move_command(
            &["/data/tools/sysctl.conf".into(), "/data/tools/fw-all.sh".into()],
            "/tmp",
        )
        .unwrap();
        assert_eq!(
            cmd,
            "mv -- '/data/tools/sysctl.conf' '/data/tools/fw-all.sh' '/tmp/'"
        );
    }

    #[test]
    fn move_skips_same_directory_sources() {
        assert!(
            build_remote_move_command(&["/data/tools/a".into()], "/data/tools").is_none()
        );
        let cmd = build_remote_move_command(
            &["/opt/a".into(), "/data/tools/b".into()],
            "/data/tools",
        )
        .unwrap();
        assert_eq!(cmd, "mv -- '/opt/a' '/data/tools/'");
    }

    #[test]
    fn same_directory_uses_copy_suffix() {
        let cmd =
            build_remote_copy_command(&["/data/tools/sysctl.conf".into()], "/data/tools").unwrap();
        assert_eq!(
            cmd,
            "cp -a -- '/data/tools/sysctl.conf' '/data/tools/sysctl.conf.copy'"
        );
    }

    #[test]
    fn mixes_same_dir_and_elsewhere() {
        let cmd = build_remote_copy_command(
            &["/opt/a".into(), "/data/tools/b".into()],
            "/data/tools",
        )
        .unwrap();
        assert_eq!(
            cmd,
            "cp -a -- '/opt/a' '/data/tools/' && cp -a -- '/data/tools/b' '/data/tools/b.copy'"
        );
    }

    #[test]
    fn zips_file_and_directory_in_listing() {
        let cmd = build_remote_zip_command(
            &[
                "/data/tools/logool".into(),
                "/data/tools/fw-all.sh".into(),
            ],
            "/data/tools",
        )
        .unwrap();
        assert_eq!(
            cmd,
            "cd -- '/data/tools' && n=1 && base='logool' && out=\"$base.zip\" && \
while [ -e \"$out\" ]; do n=$((n+1)); out=\"$base-$n.zip\"; done && zip -rq \"$out\" 'logool' 'fw-all.sh'"
        );
    }

    #[test]
    fn zip_skips_sources_outside_current_dir() {
        let cmd = build_remote_zip_command(
            &["/opt/a".into(), "/data/tools/b".into()],
            "/data/tools",
        )
        .unwrap();
        assert!(cmd.ends_with(" zip -rq \"$out\" 'b'"));
        assert!(!cmd.contains("/opt/a"));
    }

    #[test]
    fn zip_empty_when_no_local_sources() {
        assert!(build_remote_zip_command(&["/opt/a".into()], "/tmp").is_none());
        assert!(build_remote_zip_command(&[], "/tmp").is_none());
    }

    #[test]
    fn extracts_common_archives_in_the_current_directory() {
        assert_eq!(
            build_remote_extract_command("/home/gtgpu/redis-6.2.12.tar.gz", "/home/gtgpu").unwrap(),
            "cd -- '/home/gtgpu' && tar -xzf 'redis-6.2.12.tar.gz'"
        );
        assert_eq!(
            build_remote_extract_command("/home/gtgpu/redis-6.2.12.zip", "/home/gtgpu").unwrap(),
            "cd -- '/home/gtgpu' && unzip -o 'redis-6.2.12.zip'"
        );
        assert!(build_remote_extract_command("/home/gtgpu/disk.iso", "/home/gtgpu").is_none());
        assert!(build_remote_extract_command("/opt/a.tar.gz", "/home/gtgpu").is_none());
    }
}
