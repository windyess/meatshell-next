use super::*;

#[test]
fn marks_owner_and_preserves_source_tab() {
    let input = vec![
        ProcInfo {
            pid: 10,
            user: "alice".into(),
            cpu: 1.0,
            mem: 2.0,
            rss_kib: 0,
            command: "own".into(),
        },
        ProcInfo {
            pid: 11,
            user: "root".into(),
            cpu: 3.0,
            mem: 4.0,
            rss_kib: 0,
            command: "other".into(),
        },
    ];
    let rows = proc_rows(&input, "alice", "term-a");
    assert!(rows[0].own_process);
    assert!(!rows[1].own_process);
    assert!(rows.iter().all(|row| row.tab_id.as_str() == "term-a"));
}

#[test]
fn memory_sort_puts_the_largest_first() {
    let input = vec![
        ProcInfo {
            pid: 1,
            user: "a".into(),
            cpu: 9.0,
            mem: 1.0,
            rss_kib: 0,
            command: "cpu-heavy".into(),
        },
        ProcInfo {
            pid: 2,
            user: "b".into(),
            cpu: 0.1,
            mem: 40.0,
            rss_kib: 204800,
            command: "mem-heavy".into(),
        },
    ];
    let rows = ordered_proc_rows(&input, "a", "t", "mem", -1);
    assert_eq!(rows[0].pid.as_str(), "2");
    assert!(rows[0].mem_frac > rows[1].mem_frac);
    assert_eq!(rows[0].mem.as_str(), "40.0%  200.0 MB");
    assert_eq!(rows[1].mem.as_str(), "1.0%");
}

#[test]
fn command_path_opens_the_files_parent_directory() {
    assert_eq!(
        sftp_dir_from_command("python /home/gtgpu/app/main.py"),
        "/home/gtgpu/app"
    );
    assert_eq!(
        sftp_dir_from_command("nginx: master process /usr/sbin/nginx -c /etc/nginx/nginx.conf"),
        "/etc/nginx"
    );
    assert_eq!(sftp_dir_from_command("[kworker/0:1]"), "");
    assert_eq!(sftp_dir_from_command("/usr/bin/sshd"), "/usr/bin");
}

#[test]
fn privilege_rules_match_effective_login_user() {
    assert!(!process_needs_root("alice", "alice"));
    assert!(process_needs_root("alice", "root"));
    assert!(process_needs_root("alice", "bob"));
    assert!(!process_needs_root("root", "root"));
    assert!(!process_needs_root("root", "alice"));
}
