//! Turns a manifest + an action into a `Plan` for the Transaction Engine.
//! There is no per-tool logic here: everything comes from the manifest.

use crate::engine::{has_block, Plan, Step};
use crate::manifest::Manifest;
use crate::os::Os;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Install,
    Uninstall,
    Activate,
    Deactivate,
}

impl Action {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "install" => Some(Self::Install),
            "uninstall" => Some(Self::Uninstall),
            "activate" => Some(Self::Activate),
            "deactivate" => Some(Self::Deactivate),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Activate => "activate",
            Self::Deactivate => "deactivate",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PlanError {
    AlreadyInstalled,
    NotInstalled,
    NotToggleable,
    AlreadyActive,
    NotActive,
    Conflict(Vec<String>),
    Incompatible,
    UnsupportedMethod(String),
}

impl std::fmt::Display for PlanError {
    /// The UI maps the code before the colon to a translated message.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInstalled => write!(f, "already_installed"),
            Self::NotInstalled => write!(f, "not_installed"),
            Self::NotToggleable => write!(f, "not_toggleable"),
            Self::AlreadyActive => write!(f, "already_active"),
            Self::NotActive => write!(f, "not_active"),
            Self::Incompatible => write!(f, "incompatible"),
            Self::Conflict(ids) => write!(f, "conflict:{}", ids.join(",")),
            Self::UnsupportedMethod(m) => write!(f, "unsupported_method:{m}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub installed: bool,
    pub active: bool,
    /// False for tools with nothing to switch on/off (e.g. Windows Terminal).
    pub toggleable: bool,
    /// Windows build is inside the tool's supported range.
    pub compatible: bool,
}

pub fn status(os: &dyn Os, m: &Manifest) -> ToolStatus {
    let installed = m.detect.paths.iter().any(|p| os.exists(&os.expand(p)))
        || m.detect.appx.iter().any(|a| os.appx_installed(a))
        || m.detect.process.as_deref().is_some_and(|p| os.process_running(p));
    let toggleable = m.activate.is_some() && m.deactivate.is_some();
    let active = installed
        && toggleable
        && match (&m.detect.process, m.activate.as_ref().and_then(|a| a.profile_snippet.as_ref())) {
            (Some(p), _) => os.process_running(p),
            (None, Some(s)) => s
                .files
                .iter()
                .any(|f| os.read(&os.expand(f)).ok().flatten().is_some_and(|t| has_block(&t, &m.id))),
            (None, None) => false,
        };
    let build = os.windows_build();
    let compatible = build >= m.requires.windows_min_build && m.requires.windows_max_build.map_or(true, |max| build <= max);
    ToolStatus { installed, active, toggleable, compatible }
}

fn file_name(p: &str) -> &str {
    p.rsplit(['\\', '/']).next().unwrap_or(p)
}

/// Splits a manifest command into (program, args), using the real install path of the
/// program when one of `detect.paths` exists.
fn resolve(os: &dyn Os, m: &Manifest, cmd: &str) -> (String, Vec<String>) {
    let mut parts = cmd.split_whitespace();
    let prog = parts.next().unwrap_or("").to_string();
    let args: Vec<String> = parts.map(str::to_string).collect();
    let full = m
        .detect
        .paths
        .iter()
        .map(|p| os.expand(p))
        .find(|p| os.exists(p) && file_name(p).eq_ignore_ascii_case(&prog));
    (full.unwrap_or(prog), args)
}

fn autostart_name(m: &Manifest) -> String {
    format!("WinRice-{}", m.id)
}

fn winget_id(m: &Manifest) -> Result<String, PlanError> {
    match (m.install.method.as_str(), &m.install.id) {
        ("winget", Some(id)) => Ok(id.clone()),
        (other, _) => Err(PlanError::UnsupportedMethod(other.to_string())),
    }
}

fn activate_steps(os: &dyn Os, m: &Manifest) -> Vec<Step> {
    let Some(a) = &m.activate else { return vec![] };
    let mut steps = vec![];
    if let Some(c) = m.config.as_ref().filter(|c| c.backup && !c.paths.is_empty()) {
        steps.push(Step::BackupFiles { tool: m.id.clone(), files: c.paths.clone() });
    }
    if let Some(s) = &a.profile_snippet {
        steps.push(Step::AddProfileBlock { tool: m.id.clone(), files: s.files.clone(), content: s.content.clone() });
    }
    if let Some(cmd) = &a.start {
        let (program, args) = resolve(os, m, cmd);
        if a.autostart {
            let quoted = if program.contains('\\') { format!("\"{program}\"") } else { program.clone() };
            let command = std::iter::once(quoted).chain(args.iter().cloned()).collect::<Vec<_>>().join(" ");
            steps.push(Step::SetRunKey { tool: m.id.clone(), name: autostart_name(m), command });
        }
        steps.push(Step::Start { tool: m.id.clone(), program, args, process: m.detect.process.clone() });
    }
    steps
}

fn deactivate_steps(os: &dyn Os, m: &Manifest) -> Vec<Step> {
    let (Some(a), Some(d)) = (&m.activate, &m.deactivate) else { return vec![] };
    let mut steps = vec![];
    let stop = d.stop.as_ref().map(|c| resolve(os, m, c));
    if stop.is_some() || m.detect.process.is_some() {
        let restart = a.start.as_ref().map(|c| {
            let (p, args) = resolve(os, m, c);
            std::iter::once(p).chain(args).collect::<Vec<_>>()
        });
        steps.push(Step::Stop {
            tool: m.id.clone(),
            program: stop.as_ref().map(|s| s.0.clone()),
            args: stop.map(|s| s.1).unwrap_or_default(),
            process: m.detect.process.clone(),
            restart,
        });
    }
    if d.remove_autostart || a.autostart {
        steps.push(Step::DelRunKey { tool: m.id.clone(), name: autostart_name(m) });
    }
    if let Some(s) = &a.profile_snippet {
        steps.push(Step::RemoveProfileBlock { tool: m.id.clone(), files: s.files.clone() });
    }
    steps
}

pub fn plan(os: &dyn Os, all: &[Manifest], m: &Manifest, action: Action, resolve_conflicts: bool) -> Result<Plan, PlanError> {
    let st = status(os, m);
    if matches!(action, Action::Install | Action::Activate) && !st.compatible {
        return Err(PlanError::Incompatible);
    }
    let steps = match action {
        Action::Install => {
            if st.installed {
                return Err(PlanError::AlreadyInstalled);
            }
            vec![Step::WingetInstall { tool: m.id.clone(), id: winget_id(m)? }]
        }
        Action::Uninstall => {
            if !st.installed {
                return Err(PlanError::NotInstalled);
            }
            let mut s = if st.active { deactivate_steps(os, m) } else { vec![] };
            s.push(Step::WingetUninstall { tool: m.id.clone(), id: winget_id(m)? });
            s
        }
        Action::Activate => {
            if !st.installed {
                return Err(PlanError::NotInstalled);
            }
            if !st.toggleable {
                return Err(PlanError::NotToggleable);
            }
            if st.active {
                return Err(PlanError::AlreadyActive);
            }
            let clashing: Vec<String> = all
                .iter()
                .filter(|o| o.id != m.id && (m.conflicts.contains(&o.id) || o.conflicts.contains(&m.id)))
                .filter(|o| status(os, o).active)
                .map(|o| o.id.clone())
                .collect();
            let mut steps = vec![];
            if !clashing.is_empty() {
                if !resolve_conflicts {
                    return Err(PlanError::Conflict(clashing));
                }
                // The user chose to turn the clashing tools off first.
                for o in all.iter().filter(|o| clashing.contains(&o.id)) {
                    steps.extend(deactivate_steps(os, o));
                }
            }
            steps.extend(activate_steps(os, m));
            steps
        }
        Action::Deactivate => {
            if !st.toggleable {
                return Err(PlanError::NotToggleable);
            }
            if !st.active {
                return Err(PlanError::NotActive);
            }
            deactivate_steps(os, m)
        }
    };
    Ok(Plan { tool: m.id.clone(), action: action.name().into(), steps })
}

/// "Restore original Windows": switch everything off, put every backed-up original
/// config back, and (optionally) uninstall what WinRice itself installed.
pub fn plan_restore(os: &dyn Os, all: &[Manifest], installed_by_app: &[String], uninstall: bool) -> Plan {
    let mut steps: Vec<Step> = all.iter().filter(|m| status(os, m).active).flat_map(|m| deactivate_steps(os, m)).collect();
    steps.push(Step::RestoreOriginals);
    if uninstall {
        for m in all.iter().filter(|m| installed_by_app.contains(&m.id) && status(os, m).installed) {
            if let Ok(id) = winget_id(m) {
                steps.push(Step::WingetUninstall { tool: m.id.clone(), id });
            }
        }
    }
    Plan { tool: "all".into(), action: "restore".into(), steps }
}
