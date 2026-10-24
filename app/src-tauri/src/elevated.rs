//! Administrator helper.
//!
//! WinRice runs without administrator rights. A few things cannot be changed that way (for example
//! a scheduled task that a tool registered with the highest privileges). For those, WinRice starts
//! ITSELF again with the Windows "runas" prompt (UAC) and a command line such as
//!
//! ```text
//! winrice.exe --elevated task-set "\Seelen\Seelen UI Service" 0
//! ```
//!
//! The elevated copy does exactly that one thing and exits. It refuses anything that is not
//! listed here and, for tasks, anything that is not named by an official tool manifest, so it
//! cannot be used as a general "run as administrator" tool.

use crate::manifest;
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A scheduled task is only touched if an official manifest names it (`activate.autostart_task`).
pub fn allowed_task(name: &str) -> bool {
    manifest::all().iter().any(|m| m.activate.as_ref().and_then(|a| a.autostart_task.as_deref()) == Some(name))
}

/// Runs `schtasks /Change` as the current user. True when it worked.
pub fn change_task(name: &str, enabled: bool) -> bool {
    std::process::Command::new("schtasks")
        .args(["/Change", "/TN", name, if enabled { "/ENABLE" } else { "/DISABLE" }])
        .creation_flags(CREATE_NO_WINDOW)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The commands the elevated copy understands. Returns `None` for anything else.
#[derive(Debug, PartialEq)]
pub enum Command {
    TaskSet { name: String, enabled: bool },
}

pub fn parse(args: &[String]) -> Option<Command> {
    match args {
        [flag, cmd, name, value] if flag == "--elevated" && cmd == "task-set" => {
            let enabled = match value.as_str() {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            Some(Command::TaskSet { name: name.clone(), enabled })
        }
        _ => None,
    }
}

/// Decides what an `--elevated` command line does. `change` performs the task change (injected so
/// the rules can be tested without touching Windows). `None` = not a helper command line.
pub fn dispatch(args: &[String], change: impl Fn(&str, bool) -> bool) -> Option<i32> {
    if args.first().map(String::as_str) != Some("--elevated") {
        return None;
    }
    Some(match parse(args) {
        Some(Command::TaskSet { name, enabled }) if allowed_task(&name) => {
            if change(&name, enabled) { 0 } else { 3 }
        }
        Some(_) => 4, // a task no official manifest names
        None => 2,    // unknown command
    })
}

/// Called first thing in `main`: when this process was started as the helper, do the one job and
/// return the exit code (0 = done). `None` means "start the normal app".
pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    dispatch(&args, change_task)
}

/// Starts this program again with the administrator prompt and waits for it.
pub fn run_elevated(args: &[&str]) -> Result<(), String> {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};

    const ERROR_CANCELLED: u32 = 1223; // the user answered "No" to the prompt

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
    let quote = |a: &str| format!("\"{}\"", a.replace('"', ""));
    let params = std::iter::once("--elevated".to_string()).chain(args.iter().map(|a| quote(a))).collect::<Vec<_>>().join(" ");

    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let parameters = wide(&params);
    // SAFETY: SHELLEXECUTEINFOW is plain data; every pointer we put in it stays alive until after the call.
    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.nShow = 0; // SW_HIDE
        if ShellExecuteExW(&mut info) == 0 {
            return Err(if GetLastError() == ERROR_CANCELLED { "elevation_denied".into() } else { "elevation_failed".into() });
        }
        let process = info.hProcess;
        if process.is_null() {
            return Err("elevation_failed".into());
        }
        let waited = WaitForSingleObject(process, 60_000);
        let mut code: u32 = 1;
        GetExitCodeProcess(process, &mut code);
        CloseHandle(process);
        if waited != WAIT_OBJECT_0 {
            return Err("elevation_timeout".into());
        }
        if code == 0 { Ok(()) } else { Err(format!("elevated_helper_failed:{code}")) }
    }
}
