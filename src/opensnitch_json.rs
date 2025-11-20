use serde;

/// From opensnitch/daemon/ui/config/config.go
/// Only selective fields implemented to save time.
#[derive(serde::Serialize)]
#[allow(non_snake_case)]
pub struct OpenSnitchDaemonConfig {
    // LogLevel: int32,
    // Firewall: string,
    pub DefaultAction: String,
    // DefaultDuration: string,
    // ProcMonitorMethod: string ,
    // FwOptions: FwOptions,
    // Audit: audit.Config,
    // Ebpf: ebpf.Config,
    // Server: ServerConfig,
    // Rules: RulesOptions,
    // Internal: InternalOptions,
    // Stats: statistics.StatsConfig,
    // Tasks: TasksOptions,
    // InterceptUnknown: bool,
    // LogUTC: bool,
    // LogMicro: bool,
}
