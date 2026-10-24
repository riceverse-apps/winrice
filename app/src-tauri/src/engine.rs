//! Transaction Engine: plan -> backup -> apply -> verify -> commit or rollback.
//!
//! A `Plan` is a list of `Step`s. Applying a step records how to undo it; if a later
//! step fails, the recorded undos run in reverse order.

use crate::backup;
use crate::os::{winget_args, Os};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Step {
    WingetInstall { tool: String, id: String },
    WingetUninstall { tool: String, id: String },
    BackupFiles { tool: String, files: Vec<String> },
    AddProfileBlock { tool: String, files: Vec<String>, content: String },
    RemoveProfileBlock { tool: String, files: Vec<String> },
    SetRunKey { tool: String, name: String, command: String },
    DelRunKey { tool: String, name: String },
    /// `stop` (program + args) is the tool's own way to quit; it is used when the start is undone.
    Start { tool: String, program: String, args: Vec<String>, process: Option<String>, stop: Option<Vec<String>> },
    /// Stops a tool: runs `program args` if given, then kills `process` if still alive.
    /// `restart` (program + args) is used to undo the stop.
    Stop { tool: String, program: Option<String>, args: Vec<String>, process: Option<String>, restart: Option<Vec<String>> },
    RestoreOriginals,
    /// Sets one value (dotted key) inside a YAML or JSON config file.
    SetConfigValue { tool: String, file: String, format: String, key: String, value: serde_json::Value },
    /// Replaces a config file with the content shipped by a theme.
    WriteConfig { tool: String, file: String, content: String },
    /// Copies the theme wallpaper into WinRice's data folder and makes it the desktop wallpaper.
    SetWallpaper { theme: String, name: String, #[serde(skip)] bytes: Vec<u8> },
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub tool: String,
    pub action: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub ok: bool,
    pub applied: usize,
    pub error: Option<String>,
    pub rolled_back: bool,
    pub rollback_errors: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub enum Undo {
    WriteFile { path: String, prev: Option<String> },
    WingetUninstall(String),
    WingetInstall(String),
    RunKey { name: String, prev: Option<String> },
    Kill(String),
    /// Stop a process the proper way (the tool's stop command first, then force) and check it is gone.
    StopProcess { stop: Option<Vec<String>>, process: String },
    Spawn(Vec<String>),
    Wallpaper { prev: Option<String> },
}

/// Everything needed to undo a plan that already succeeded (try mode, watchdog, shortcut).
#[derive(Default, Serialize, Deserialize)]
pub struct Journal(Vec<Undo>);

impl Journal {
    pub fn extend(&mut self, other: Journal) {
        self.0.extend(other.0);
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "[]".into())
    }
    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

fn marker(tool: &str, open: bool) -> String {
    if open { format!("# >>> winrice:{tool} >>>") } else { format!("# <<< winrice:{tool} <<<") }
}

pub fn has_block(text: &str, tool: &str) -> bool {
    text.contains(&marker(tool, true))
}

/// Text with the WinRice block for `tool` removed (unchanged when absent).
pub fn without_block(text: &str, tool: &str) -> String {
    let (open, close) = (marker(tool, true), marker(tool, false));
    let Some(start) = text.find(&open) else { return text.to_string() };
    let end = text[start..].find(&close).map(|e| start + e + close.len()).unwrap_or(text.len());
    let tail = text[end..].trim_start_matches(['\r', '\n']);
    let head = &text[..start];
    format!("{head}{tail}")
}

pub fn with_block(text: &str, tool: &str, content: &str) -> String {
    let mut base = without_block(text, tool);
    if !base.is_empty() && !base.ends_with('\n') {
        base.push('\n');
    }
    format!("{base}{}\n{content}\n{}\n", marker(tool, true), marker(tool, false))
}

/// winget exit codes that mean "this package is already installed and up to date"
/// (0x8A15002B: no applicable update, 0x8A150061: package already installed).
const ALREADY_INSTALLED: [i32; 2] = [-1978335189, -1978335135];

/// Runs winget. Returns true when `install` found the package already there.
fn winget(os: &dyn Os, verb: &str, id: &str) -> Result<bool, String> {
    let code = os.run("winget", &winget_args(verb, id))?;
    if code == 0 {
        Ok(false)
    } else if verb == "install" && ALREADY_INSTALLED.contains(&code) {
        Ok(true)
    } else {
        Err(format!("winget {verb} {id} failed (exit code {code})"))
    }
}

fn wait_for(os: &dyn Os, process: &str, want_running: bool) -> bool {
    for _ in 0..10 {
        if os.process_running(process) == want_running {
            return true;
        }
        os.sleep_ms(500);
    }
    os.process_running(process) == want_running
}

/// Stops a process for real. Some tools restart themselves when they are simply killed (a
/// helper service keeps them alive), so the tool's own stop command runs first; then, if the
/// process is still there, it is killed; and finally we CHECK that it is gone.
fn stop_process(os: &dyn Os, stop: Option<&[String]>, process: &str) -> Result<(), String> {
    if let Some((program, args)) = stop.and_then(|s| s.split_first()) {
        os.run(program, args)?;
    }
    if !wait_for(os, process, false) {
        os.kill_process(process)?;
        if !wait_for(os, process, false) {
            return Err(format!("{process} did not stop"));
        }
    }
    Ok(())
}

fn apply(os: &dyn Os, step: &Step, undos: &mut Vec<Undo>) -> Result<(), String> {
    match step {
        Step::WingetInstall { id, .. } => {
            undos.push(Undo::WingetUninstall(id.clone()));
            // Already there: fine, but it was not WinRice that installed it, so undoing must not remove it.
            if winget(os, "install", id)? {
                undos.pop();
            }
            Ok(())
        }
        Step::WingetUninstall { id, .. } => {
            winget(os, "uninstall", id)?;
            undos.push(Undo::WingetInstall(id.clone()));
            Ok(())
        }
        Step::BackupFiles { files, .. } => files.iter().try_for_each(|f| backup::backup_file(os, &os.expand(f))),
        Step::AddProfileBlock { tool, files, content } => {
            for f in files {
                let path = os.expand(f);
                backup::backup_file(os, &path)?;
                let prev = os.read(&path).map_err(|e| format!("{path}: {e}"))?;
                let next = with_block(prev.as_deref().unwrap_or(""), tool, content);
                undos.push(Undo::WriteFile { path: path.clone(), prev });
                os.write(&path, &next).map_err(|e| format!("{path}: {e}"))?;
            }
            Ok(())
        }
        Step::RemoveProfileBlock { tool, files } => {
            for f in files {
                let path = os.expand(f);
                let Some(prev) = os.read(&path).map_err(|e| format!("{path}: {e}"))? else { continue };
                if !has_block(&prev, tool) {
                    continue;
                }
                backup::backup_file(os, &path)?;
                let next = without_block(&prev, tool);
                undos.push(Undo::WriteFile { path: path.clone(), prev: Some(prev) });
                os.write(&path, &next).map_err(|e| format!("{path}: {e}"))?;
            }
            Ok(())
        }
        Step::SetRunKey { name, command, .. } => {
            undos.push(Undo::RunKey { name: name.clone(), prev: os.run_key_get(name) });
            os.run_key_set(name, command)
        }
        Step::DelRunKey { name, .. } => {
            if let Some(prev) = os.run_key_get(name) {
                undos.push(Undo::RunKey { name: name.clone(), prev: Some(prev) });
                os.run_key_del(name)?;
            }
            Ok(())
        }
        Step::Start { program, args, process, stop, .. } => {
            if let Some(p) = process {
                undos.push(Undo::StopProcess { stop: stop.clone(), process: p.clone() });
            }
            os.spawn(program, args)?;
            match process {
                // Verify: the process must actually come up.
                Some(p) if !wait_for(os, p, true) => Err(format!("{p} did not start")),
                _ => Ok(()),
            }
        }
        Step::Stop { program, args, process, restart, .. } => {
            if let Some(r) = restart {
                undos.push(Undo::Spawn(r.clone()));
            }
            let cmd: Option<Vec<String>> = program.as_ref().map(|p| std::iter::once(p.clone()).chain(args.iter().cloned()).collect());
            match process {
                Some(p) => stop_process(os, cmd.as_deref(), p),
                None => match cmd.as_deref().and_then(|c| c.split_first()) {
                    Some((prog, a)) => os.run(prog, a).map(|_| ()),
                    None => Ok(()),
                },
            }
        }
        Step::SetConfigValue { file, format, key, value, .. } => {
            let path = os.expand(file);
            backup::backup_file(os, &path)?;
            let prev = os.read(&path).map_err(|e| format!("{path}: {e}"))?;
            let next = crate::settings::config_set(prev.as_deref().unwrap_or(""), format, key, value)?;
            undos.push(Undo::WriteFile { path: path.clone(), prev });
            os.write(&path, &next).map_err(|e| format!("{path}: {e}"))
        }
        Step::WriteConfig { file, content, .. } => {
            let path = os.expand(file);
            backup::backup_file(os, &path)?;
            let prev = os.read(&path).map_err(|e| format!("{path}: {e}"))?;
            undos.push(Undo::WriteFile { path: path.clone(), prev });
            os.write(&path, content).map_err(|e| format!("{path}: {e}"))
        }
        Step::SetWallpaper { theme, name, bytes } => {
            let path = format!("{}\\wallpapers\\{theme}-{name}", os.data_dir());
            os.write_bytes(&path, bytes).map_err(|e| format!("{path}: {e}"))?;
            undos.push(Undo::Wallpaper { prev: os.wallpaper_get() });
            os.wallpaper_set(&path)
        }
        Step::RestoreOriginals => {
            for (path, prev) in backup::restore_originals(os)? {
                undos.push(Undo::WriteFile { path, prev });
            }
            Ok(())
        }
    }
}

fn undo(os: &dyn Os, u: &Undo) -> Result<(), String> {
    match u {
        Undo::WriteFile { path, prev: Some(c) } => os.write(path, c).map_err(|e| format!("{path}: {e}")),
        Undo::WriteFile { path, prev: None } => os.remove_file(path).map_err(|e| format!("{path}: {e}")),
        Undo::WingetUninstall(id) => winget(os, "uninstall", id).map(|_| ()),
        Undo::WingetInstall(id) => winget(os, "install", id).map(|_| ()),
        Undo::RunKey { name, prev: Some(c) } => os.run_key_set(name, c),
        Undo::RunKey { name, prev: None } => os.run_key_del(name),
        Undo::Kill(p) => os.kill_process(p),
        Undo::StopProcess { stop, process } => stop_process(os, stop.as_deref(), process),
        Undo::Wallpaper { prev: Some(p) } => os.wallpaper_set(p),
        // There was no wallpaper before: go back to the plain background color.
        Undo::Wallpaper { prev: None } => os.wallpaper_set(""),
        Undo::Spawn(cmd) => match cmd.split_first() {
            Some((prog, args)) => os.spawn(prog, args),
            None => Ok(()),
        },
    }
}

fn rollback(os: &dyn Os, undos: &[Undo]) -> Vec<String> {
    undos.iter().rev().filter_map(|u| undo(os, u).err()).collect()
}

/// Undoes a plan that already succeeded. Returns the errors, if any.
pub fn revert(os: &dyn Os, journal: &Journal) -> Vec<String> {
    rollback(os, &journal.0)
}

/// Runs the plan. On the first failing step, rolls back everything already done.
/// On success the journal can be kept to undo the whole plan later.
pub fn run_journal(os: &dyn Os, plan: &Plan) -> (Outcome, Journal) {
    let mut undos: Vec<Undo> = Vec::new();
    let mut applied = 0;
    for step in &plan.steps {
        if let Err(error) = apply(os, step, &mut undos) {
            let rollback_errors = rollback(os, &undos);
            let out = Outcome { ok: false, applied, error: Some(error), rolled_back: true, rollback_errors };
            return (out, Journal(vec![]));
        }
        applied += 1;
    }
    (Outcome { ok: true, applied, error: None, rolled_back: false, rollback_errors: vec![] }, Journal(undos))
}

#[cfg(test)]
pub fn run(os: &dyn Os, plan: &Plan) -> Outcome {
    run_journal(os, plan).0
}
