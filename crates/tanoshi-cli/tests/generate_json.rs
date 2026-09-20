use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use tanoshi_vm::PLUGIN_EXTENSION;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

fn executable_name() -> String {
    format!("tanoshi-cli{}", std::env::consts::EXE_SUFFIX)
}

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "tanoshi-cli-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("plugins")).unwrap();
        // Exercise the shipped, single-binary layout without a sibling worker.
        fs::copy(
            env!("CARGO_BIN_EXE_tanoshi-cli"),
            path.join(executable_name()),
        )
        .unwrap();
        Self(path)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.0.join(executable_name()));
        command
            .current_dir(&self.0)
            .env_remove("TANOSHI_EXTENSION_WORKER")
            .env("PATH", "")
            .args(["--path", "plugins", "generate-json"]);
        command
    }

    fn index_path(&self) -> PathBuf {
        self.0
            .join("output")
            .join(env!("TARGET"))
            .join("index.json")
    }

    fn broken_plugin(&self) {
        fs::write(
            self.0
                .join("plugins")
                .join(format!("broken.{PLUGIN_EXTENSION}")),
            b"invalid dynamic library",
        )
        .unwrap();
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn failure_stderr(output: Output) -> String {
    assert!(
        !output.status.success(),
        "generation unexpectedly succeeded"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn empty_input_does_not_publish_an_empty_index() {
    let workspace = Workspace::new();
    let stderr = failure_stderr(workspace.command().output().unwrap());
    assert!(stderr.contains("no extensions found"), "{stderr}");
    assert!(!workspace.index_path().exists());
}

#[test]
fn bundled_worker_load_failure_preserves_previous_index() {
    let workspace = Workspace::new();
    workspace.broken_plugin();
    let index = workspace.index_path();
    fs::create_dir_all(index.parent().unwrap()).unwrap();
    let previous_index = r#"[{"id":123,"name":"Previous source"}]"#;
    fs::write(&index, previous_index).unwrap();

    let stderr = failure_stderr(workspace.command().output().unwrap());
    assert!(stderr.contains("failed to load broken"), "{stderr}");
    // The child started and attempted loading the library, then closed its pipe.
    // A missing standalone worker would instead fail at process creation.
    assert!(
        stderr.contains("failed to read extension worker readiness"),
        "{stderr}"
    );
    assert_eq!(fs::read_to_string(index).unwrap(), previous_index);
}

#[test]
fn explicit_worker_override_is_respected_and_failure_is_fatal() {
    let workspace = Workspace::new();
    workspace.broken_plugin();
    let stderr = failure_stderr(
        workspace
            .command()
            .env(
                "TANOSHI_EXTENSION_WORKER",
                workspace.0.join("missing-worker"),
            )
            .output()
            .unwrap(),
    );
    assert!(stderr.contains("missing-worker"), "{stderr}");
    assert!(stderr.contains("source index was not written"), "{stderr}");
    assert!(!workspace.index_path().exists());
}
