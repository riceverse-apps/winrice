//! Everything that touches the machine goes through the `Os` trait, so the
//! Transaction Engine can be tested with an in-memory fake (no real installs).

use std::io;

pub trait Os: Send + Sync {
    fn expand(&self, s: &str) -> String;
    fn exists(&self, path: &str) -> bool;
    /// `None` when the file does not exist.
    fn read(&self, path: &str) -> io::Result<Option<String>>;
    fn write(&self, path: &str, content: &str) -> io::Result<()>;
    fn remove_file(&self, path: &str) -> io::Result<()>;
    fn write_bytes(&self, path: &str, bytes: &[u8]) -> io::Result<()>;
    fn wallpaper_get(&self) -> Option<String>;
    fn wallpaper_set(&self, path: &str) -> Result<(), String>;
    fn list_dir(&self, dir: &str) -> Vec<String>;
    /// Runs a program and waits; returns the exit code.
    fn run(&self, program: &str, args: &[String]) -> Result<i32, String>;
    /// Starts a program detached (it survives WinRice closing).
    fn spawn(&self, program: &str, args: &[String]) -> Result<(), String>;
    fn process_running(&self, name: &str) -> bool;
    fn kill_process(&self, name: &str) -> Result<(), String>;
    fn run_key_get(&self, name: &str) -> Option<String>;
    fn run_key_set(&self, name: &str, command: &str) -> Result<(), String>;
    fn run_key_del(&self, name: &str) -> Result<(), String>;
    fn process_ids(&self, name: &str) -> Vec<u32>;
    /// Is an MSIX/Appx package with this name (e.g. `Seelen.SeelenUI`) installed for the current user?
    fn appx_installed(&self, name: &str) -> bool;
    /// Package family name (`Name_publisherid`) of an installed MSIX package.
    fn appx_family(&self, name: &str) -> Option<String>;
    fn windows_build(&self) -> u32;
    fn windows_edition(&self) -> String;
    fn data_dir(&self) -> String;
    fn now(&self) -> u64;
    fn sleep_ms(&self, ms: u64);
}

pub fn winget_args(verb: &str, id: &str) -> Vec<String> {
    let mut a = vec![verb.to_string(), "--id".into(), id.into(), "-e".into(), "--silent".into()];
    if verb == "install" {
        a.push("--accept-package-agreements".into());
        a.push("--accept-source-agreements".into());
    }
    a
}

#[cfg(windows)]
pub use real::RealOs;

#[cfg(windows)]
mod real {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::path::Path;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    pub struct RealOs;

    impl RealOs {
        fn nt_value(&self, name: &str) -> Option<String> {
            let out = hidden(
                "reg",
                &["query".into(), r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion".into(), "/v".into(), name.into()],
            )
            .output()
            .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines().find(|l| l.trim_start().starts_with(name)).and_then(|l| l.split_once("REG_SZ")).map(|(_, v)| v.trim().to_string())
        }
    }

    fn hidden(program: &str, args: &[String]) -> Command {
        let mut c = Command::new(program);
        c.args(args).creation_flags(CREATE_NO_WINDOW);
        c
    }

    impl Os for RealOs {
        fn expand(&self, s: &str) -> String {
            let mut out = String::new();
            let mut rest = s;
            while let Some(i) = rest.find('%') {
                out.push_str(&rest[..i]);
                let after = &rest[i + 1..];
                match after.find('%') {
                    Some(j) => {
                        let name = &after[..j];
                        match std::env::var(name) {
                            Ok(v) => out.push_str(&v),
                            Err(_) => {
                                out.push('%');
                                out.push_str(name);
                                out.push('%');
                            }
                        }
                        rest = &after[j + 1..];
                    }
                    None => {
                        out.push('%');
                        rest = after;
                    }
                }
            }
            out.push_str(rest);
            out
        }

        fn exists(&self, path: &str) -> bool {
            Path::new(path).exists()
        }

        fn read(&self, path: &str) -> io::Result<Option<String>> {
            match std::fs::read_to_string(path) {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e),
            }
        }

        fn write(&self, path: &str, content: &str) -> io::Result<()> {
            if let Some(p) = Path::new(path).parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(path, content)
        }

        fn remove_file(&self, path: &str) -> io::Result<()> {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                r => r,
            }
        }

        fn write_bytes(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
            if let Some(p) = Path::new(path).parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(path, bytes)
        }

        fn wallpaper_get(&self) -> Option<String> {
            let out = hidden("reg", &["query".into(), r"HKCU\Control Panel\Desktop".into(), "/v".into(), "WallPaper".into()]).output().ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .find(|l| l.trim_start().starts_with("WallPaper"))
                .and_then(|l| l.split_once("REG_SZ"))
                .map(|(_, v)| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }

        // UNVERIFIED on a clean VM: registry value + UpdatePerUserSystemParameters.
        fn wallpaper_set(&self, path: &str) -> Result<(), String> {
            let code = self.run(
                "reg",
                &["add".into(), r"HKCU\Control Panel\Desktop".into(), "/v".into(), "WallPaper".into(), "/t".into(), "REG_SZ".into(), "/d".into(), path.into(), "/f".into()],
            )?;
            if code != 0 {
                return Err(format!("reg add failed ({code})"));
            }
            self.run("RUNDLL32.EXE", &["user32.dll,UpdatePerUserSystemParameters".into(), "1".into(), "True".into()]).map(|_| ())
        }

        fn list_dir(&self, dir: &str) -> Vec<String> {
            std::fs::read_dir(dir)
                .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path().to_string_lossy().into_owned()).collect())
                .unwrap_or_default()
        }

        fn run(&self, program: &str, args: &[String]) -> Result<i32, String> {
            hidden(program, args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.code().unwrap_or(-1))
                .map_err(|e| format!("{program}: {e}"))
        }

        fn spawn(&self, program: &str, args: &[String]) -> Result<(), String> {
            let mut c = Command::new(program);
            c.args(args)
                .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            c.spawn().map(|_| ()).map_err(|e| format!("{program}: {e}"))
        }

        fn process_running(&self, name: &str) -> bool {
            let out = hidden("tasklist", &["/FI".into(), format!("IMAGENAME eq {name}"), "/NH".into(), "/FO".into(), "CSV".into()]).output();
            out.map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&name.to_lowercase()))
                .unwrap_or(false)
        }

        fn kill_process(&self, name: &str) -> Result<(), String> {
            // taskkill exits non-zero when no such process exists; that is fine.
            self.run("taskkill", &["/IM".into(), name.into(), "/F".into()]).map(|_| ())
        }

        fn run_key_get(&self, name: &str) -> Option<String> {
            let out = hidden("reg", &["query".into(), RUN_KEY.into(), "/v".into(), name.into()]).output().ok()?;
            if !out.status.success() {
                return None;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines().find(|l| l.trim_start().starts_with(name)).and_then(|l| {
                l.split_once("REG_SZ").map(|(_, v)| v.trim().to_string())
            })
        }

        fn run_key_set(&self, name: &str, command: &str) -> Result<(), String> {
            let code = self.run(
                "reg",
                &["add".into(), RUN_KEY.into(), "/v".into(), name.into(), "/t".into(), "REG_SZ".into(), "/d".into(), command.into(), "/f".into()],
            )?;
            if code == 0 { Ok(()) } else { Err(format!("reg add failed ({code})")) }
        }

        fn run_key_del(&self, name: &str) -> Result<(), String> {
            let code = self.run("reg", &["delete".into(), RUN_KEY.into(), "/v".into(), name.into(), "/f".into()])?;
            if code == 0 { Ok(()) } else { Err(format!("reg delete failed ({code})")) }
        }

        fn appx_family(&self, name: &str) -> Option<String> {
            let key = r"HKCU\Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";
            let out = hidden("reg", &["query".into(), key.into(), "/f".into(), format!("{name}_"), "/k".into()]).output().ok()?;
            let text = String::from_utf8_lossy(&out.stdout).into_owned();
            let prefix = format!("\\Packages\\{name}_").to_lowercase();
            // ...\Packages\Seelen.SeelenUI_2.8.8.0_x64__p6yyn03m1894e  ->  Seelen.SeelenUI_p6yyn03m1894e
            text.lines().find(|l| l.to_lowercase().contains(&prefix)).and_then(|l| {
                let full = l.trim().rsplit('\\').next()?;
                let publisher = full.rsplit_once("__")?.1;
                Some(format!("{name}_{publisher}"))
            })
        }

        fn appx_installed(&self, name: &str) -> bool {
            // MSIX packages are registered per user here; reading it is fast and needs no admin.
            let key = r"HKCU\Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";
            let out = hidden("reg", &["query".into(), key.into(), "/f".into(), format!("{name}_"), "/k".into()]).output();
            let needle = format!("\\packages\\{}_", name.to_lowercase());
            out.map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains(&needle)).unwrap_or(false)
        }

        fn process_ids(&self, name: &str) -> Vec<u32> {
            let out = hidden("tasklist", &["/FI".into(), format!("IMAGENAME eq {name}"), "/NH".into(), "/FO".into(), "CSV".into()]).output();
            let Ok(out) = out else { return vec![] };
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| l.split("\",\"").nth(1).and_then(|pid| pid.trim_matches('"').parse().ok()))
                .collect()
        }

        fn windows_build(&self) -> u32 {
            self.nt_value("CurrentBuildNumber").and_then(|v| v.parse().ok()).unwrap_or(0)
        }

        fn windows_edition(&self) -> String {
            self.nt_value("EditionID").unwrap_or_default()
        }

        fn data_dir(&self) -> String {
            format!("{}\\WinRice", std::env::var("APPDATA").unwrap_or_else(|_| ".".into()))
        }

        fn now(&self) -> u64 {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
        }

        fn sleep_ms(&self, ms: u64) {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Mutex;

    #[derive(Default, Clone, PartialEq, Debug)]
    pub struct World {
        pub files: BTreeMap<String, String>,
        pub run_keys: BTreeMap<String, String>,
        pub processes: BTreeSet<String>,
        pub winget: BTreeSet<String>,
        pub wallpaper: Option<String>,
        pub appx: BTreeSet<String>,
    }

    pub struct FakeOs {
        pub world: Mutex<World>,
        ticks: Mutex<usize>,
        pub fail_at: Mutex<Option<usize>>,
        pub build: Mutex<u32>,
        /// winget id -> files that appear when it is installed (and vanish when uninstalled).
        pub packages: Mutex<BTreeMap<String, Vec<String>>>,
        /// Process ids returned for every process name (Explorer watchdog tests).
        pub pids: Mutex<Vec<u32>>,
        /// package family -> process that `explorer.exe shell:AppsFolder\<family>!App` starts.
        pub appx_launch: Mutex<BTreeMap<String, String>>,
        /// winget id -> MSIX package name registered when it is installed (and removed when uninstalled).
        pub appx_packages: Mutex<BTreeMap<String, String>>,
        /// When true, spawned programs never show up as running processes.
        pub spawn_is_dud: Mutex<bool>,
    }

    impl FakeOs {
        pub fn new(world: World) -> Self {
            Self { world: Mutex::new(world), ticks: Mutex::new(0), fail_at: Mutex::new(None), spawn_is_dud: Mutex::new(false), build: Mutex::new(26100), packages: Mutex::new(BTreeMap::new()), appx_launch: Mutex::new(BTreeMap::new()), appx_packages: Mutex::new(BTreeMap::new()), pids: Mutex::new(vec![]) }
        }
        /// Counts a mutating call; fails exactly once when it reaches `fail_at`.
        fn tick(&self) -> Result<(), String> {
            let mut t = self.ticks.lock().unwrap();
            let n = *t;
            *t += 1;
            let mut f = self.fail_at.lock().unwrap();
            if *f == Some(n) {
                *f = None;
                return Err("injected failure".into());
            }
            Ok(())
        }
        pub fn read_for_test(&self, path: &str) -> String {
            self.world.lock().unwrap().files.get(path).cloned().unwrap_or_default()
        }
        pub fn ticks(&self) -> usize {
            *self.ticks.lock().unwrap()
        }
        /// World without WinRice's own data dir, to compare machine state before/after.
        pub fn machine(&self) -> World {
            let mut w = self.world.lock().unwrap().clone();
            w.files.retain(|k, _| !k.starts_with("DATA"));
            w
        }
    }

    fn io_err(m: String) -> io::Error {
        io::Error::new(io::ErrorKind::Other, m)
    }

    impl Os for FakeOs {
        fn expand(&self, s: &str) -> String {
            s.to_string()
        }
        fn exists(&self, path: &str) -> bool {
            self.world.lock().unwrap().files.contains_key(path)
        }
        fn read(&self, path: &str) -> io::Result<Option<String>> {
            Ok(self.world.lock().unwrap().files.get(path).cloned())
        }
        fn write(&self, path: &str, content: &str) -> io::Result<()> {
            self.tick().map_err(io_err)?;
            self.world.lock().unwrap().files.insert(path.into(), content.into());
            Ok(())
        }
        fn remove_file(&self, path: &str) -> io::Result<()> {
            self.tick().map_err(io_err)?;
            self.world.lock().unwrap().files.remove(path);
            Ok(())
        }
        fn write_bytes(&self, path: &str, bytes: &[u8]) -> io::Result<()> {
            self.tick().map_err(io_err)?;
            self.world.lock().unwrap().files.insert(path.into(), String::from_utf8_lossy(bytes).into_owned());
            Ok(())
        }
        fn wallpaper_get(&self) -> Option<String> {
            self.world.lock().unwrap().wallpaper.clone()
        }
        fn wallpaper_set(&self, path: &str) -> Result<(), String> {
            self.tick()?;
            self.world.lock().unwrap().wallpaper = if path.is_empty() { None } else { Some(path.into()) };
            Ok(())
        }
        fn list_dir(&self, dir: &str) -> Vec<String> {
            let prefix = format!("{dir}\\");
            self.world.lock().unwrap().files.keys().filter(|k| k.starts_with(&prefix)).cloned().collect()
        }
        fn run(&self, program: &str, args: &[String]) -> Result<i32, String> {
            self.tick()?;
            let mut w = self.world.lock().unwrap();
            if let Some(id) = args.iter().position(|a| a == "--id").and_then(|i| args.get(i + 1)) {
                let files = self.packages.lock().unwrap().get(id).cloned().unwrap_or_default();
                let appx = self.appx_packages.lock().unwrap().get(id).cloned();
                if args[0] == "install" && w.winget.contains(id) {
                    return Ok(-1978335189); // 0x8A15002B: already installed, no update
                }
                match args[0].as_str() {
                    "install" => {
                        for f in files {
                            w.files.insert(f, String::new());
                        }
                        if let Some(a) = appx {
                            w.appx.insert(a);
                        }
                        w.winget.insert(id.clone())
                    }
                    _ => {
                        for f in files {
                            w.files.remove(&f);
                        }
                        if let Some(a) = appx {
                            w.appx.remove(&a);
                        }
                        w.winget.remove(id)
                    }
                };
            } else if args.iter().any(|a| a == "wm-exit") {
                w.processes.remove("glazewm.exe");
            } else if program.ends_with("komorebic.exe") && args.first().map(String::as_str) == Some("stop") {
                // komorebic is the CLI that stops komorebi.exe
                w.processes.remove("komorebi.exe");
            }
            let _ = program;
            Ok(0)
        }
        fn spawn(&self, program: &str, _args: &[String]) -> Result<(), String> {
            self.tick()?;
            if !*self.spawn_is_dud.lock().unwrap() {
                if let Some(family) = _args.first().and_then(|a| a.strip_prefix("shell:AppsFolder\\")).and_then(|a| a.split('!').next()) {
                    if let Some(proc_name) = self.appx_launch.lock().unwrap().get(family).cloned() {
                        self.world.lock().unwrap().processes.insert(proc_name);
                    }
                    return Ok(());
                }
                let name = program.rsplit('\\').next().unwrap_or(program).to_string();
                let mut w = self.world.lock().unwrap();
                if name == "komorebic.exe" {
                    // the CLI launches the real process and exits right away
                    w.processes.insert("komorebi.exe".into());
                } else {
                    w.processes.insert(name);
                }
            }
            Ok(())
        }
        fn process_running(&self, name: &str) -> bool {
            self.world.lock().unwrap().processes.contains(name)
        }
        fn kill_process(&self, name: &str) -> Result<(), String> {
            self.tick()?;
            self.world.lock().unwrap().processes.remove(name);
            Ok(())
        }
        fn run_key_get(&self, name: &str) -> Option<String> {
            self.world.lock().unwrap().run_keys.get(name).cloned()
        }
        fn run_key_set(&self, name: &str, command: &str) -> Result<(), String> {
            self.tick()?;
            self.world.lock().unwrap().run_keys.insert(name.into(), command.into());
            Ok(())
        }
        fn run_key_del(&self, name: &str) -> Result<(), String> {
            self.tick()?;
            self.world.lock().unwrap().run_keys.remove(name);
            Ok(())
        }
        fn appx_installed(&self, name: &str) -> bool {
            self.world.lock().unwrap().appx.contains(name)
        }
        fn appx_family(&self, name: &str) -> Option<String> {
            self.appx_installed(name).then(|| format!("{name}_fakepub"))
        }
        fn process_ids(&self, _name: &str) -> Vec<u32> {
            self.pids.lock().unwrap().clone()
        }
        fn windows_build(&self) -> u32 {
            *self.build.lock().unwrap()
        }
        fn windows_edition(&self) -> String {
            "Professional".into()
        }
        fn data_dir(&self) -> String {
            "DATA".into()
        }
        fn now(&self) -> u64 {
            1_700_000_000
        }
        fn sleep_ms(&self, _ms: u64) {}
    }
}
