/// One process row (#23). CPU is the last sample interval, not the lifetime
/// average; memory is the resident share. `command` is the truncated args.
#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: u32,
    pub user: String,
    pub cpu: f32,
    pub mem: f32,
    pub command: String,
}

#[derive(Debug, Clone, Default)]
pub struct SystemDetails {
    pub overview: Vec<(String, String)>,
    pub cpu_info: Vec<(String, String)>,
    pub gpu_info: Vec<(String, String)>,
    pub cpu_usage: Vec<(String, String)>,
    pub memory: Vec<(String, String)>,
    pub swap: Vec<(String, String)>,
    pub networks: Vec<(String, String, String, String, String)>,
    pub filesystems: Vec<(String, String, String, String, String)>,
}

/// One SSH tunnel row shown in the runtime tunnel panel (#206).
#[derive(Debug, Clone)]
pub struct RuntimeTunnelInfo {
    pub id: String,
    pub name: String,
    /// The name the user typed. Empty when `name` is a generated label.
    pub rule_name: String,
    pub kind: String,
    pub bind_addr: String,
    pub bind_port: u16,
    pub host: String,
    pub host_port: u16,
    pub auto_start: bool,
    /// Stored on the SSH session (`saved-` / `config-`), not a runtime-only rule.
    pub saved: bool,
    pub active: bool,
    /// Listen or remote-forward request failed. The row stays stopped.
    pub failed: bool,
    pub status: String,
}
