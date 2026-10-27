//! The linking code travels over a private child stdin pipe, never argv or logs.
use std::{path::Path, sync::mpsc};
use serde_json::{json, Value};

pub(super) fn start(root: &Path, code: String) -> mpsc::Receiver<Value> {run(root,Some(code))}
pub(super) fn logout(root:&Path)->mpsc::Receiver<Value>{run(root,None)}
fn run(root:&Path,code:Option<String>)->mpsc::Receiver<Value> {
    let project = crate::frontend::account_preview::helper_project(root);
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        use std::{io::Write, process::{Command, Stdio}};
        let failure = || json!({"ok":false,"message":"Sign-in helper failed. No automatic retry was made."});
        let run = || -> Option<Value> {
            let bundled = project.join("tools/.venv/Scripts/python.exe");
            let runtime = if bundled.is_file() { bundled } else { "python".into() };
            let mut command = Command::new(runtime);
            command.current_dir(&project).arg("-I").arg(project.join("tools/assets/sign_in_ui.py"))

                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
            if code.is_some(){command.env("RRBTD6_NK_ONLINE","1");}else{command.arg("--logout");}
            #[cfg(windows)] {
                use std::os::windows::process::CommandExt;
                command.creation_flags(0x08000000);
            }
            let mut child = command.spawn().ok()?;
            // Always close stdin so a helper waiting for EOF can finish.
            let payload = serde_json::to_vec(&json!({"code":code})).ok()?;
            let sent = child.stdin.take().is_some_and(|mut input| input.write_all(&payload).is_ok());
            if !sent { let _=child.kill(); let _=child.wait(); return None; }
            let output = child.wait_with_output().ok()?;
            if !output.status.success() || output.stdout.len()>4*1024*1024 { return None; }
            serde_json::from_slice(&output.stdout).ok()
        };
        let _=tx.send(run().unwrap_or_else(failure));
    });
    rx
}
