use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tempfile::TempDir;

/// Test harness: creates a bare "remote" and a working "local" clone.
struct WipTestEnv {
    _tempdir: TempDir,
    local: PathBuf,
}

impl WipTestEnv {
    fn new() -> Self {
        let tempdir = TempDir::new().expect("failed to create temp dir");
        let base = tempdir.path();
        let remote = base.join("remote.git");
        let local = base.join("local");

        // Create bare remote
        git(base, &["init", "--bare", remote.to_str().unwrap()]);

        // Clone it
        git(base, &["clone", remote.to_str().unwrap(), "local"]);

        // Configure user identity in the local clone
        git(&local, &["config", "user.name", "Test User"]);
        git(&local, &["config", "user.email", "test@example.com"]);

        // Create an initial commit so HEAD exists
        fs::write(local.join("README"), "init").unwrap();
        git(&local, &["add", "README"]);
        git(&local, &["commit", "-m", "initial commit"]);

        WipTestEnv {
            _tempdir: tempdir,
            local,
        }
    }

    /// Run the `wip` binary in the local repo, returning the Command for assertions.
    fn wip(&self, args: &[&str]) -> Command {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("wip");
        cmd.current_dir(&self.local);
        cmd.args(args);
        cmd
    }

    /// Create a dirty file in the working tree.
    fn dirty(&self, name: &str, content: &str) {
        fs::write(self.local.join(name), content).unwrap();
    }

    /// Read a file from the working tree.
    fn read_file(&self, name: &str) -> String {
        fs::read_to_string(self.local.join(name)).unwrap()
    }

    /// Check if a file exists in the working tree.
    fn file_exists(&self, name: &str) -> bool {
        self.local.join(name).exists()
    }
}

/// Run a raw git command in the given directory.
fn git(dir: &Path, args: &[&str]) -> String {
    git_env(dir, args, &[])
}

/// Run a raw git command with extra environment variables.
fn git_env(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> String {
    let mut cmd = process::Command::new("git");
    cmd.args(args).current_dir(dir);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("failed to run git");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("git {} failed: {}", args.join(" "), stderr);
    }

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

// ── Integration tests ───────────────────────────────────────────────

#[test]
fn test_save_and_list() {
    let env = WipTestEnv::new();
    env.dirty("foo.txt", "hello");

    env.wip(&["save", "my-wip", "-m", "test save"])
        .assert()
        .success()
        .stdout(contains("saved"));

    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("my-wip"));
}

#[test]
fn test_save_and_load() {
    let env = WipTestEnv::new();
    env.dirty("work.txt", "important work");

    // Save
    env.wip(&["save", "restore-test", "-m", "will restore", "--force"])
        .assert()
        .success();

    // Discard local changes
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);
    assert!(!env.file_exists("work.txt"));

    // Load
    env.wip(&["load", "restore-test"])
        .assert()
        .success()
        .stdout(contains("loaded"));

    assert_eq!(env.read_file("work.txt"), "important work");
}

#[test]
fn test_save_and_show() {
    let env = WipTestEnv::new();
    env.dirty("show-me.txt", "visible content");

    env.wip(&["save", "show-test", "-m", "showing", "--force"])
        .assert()
        .success();

    // Discard so show can fetch cleanly
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    let output = env.wip(&["show", "show-test"]).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("show-test"), "should contain the name");
    assert!(stdout.contains("showing"), "should contain the message");
    assert!(
        stdout.contains("show-me.txt"),
        "should contain the filename in the diff"
    );
}

#[test]
fn test_save_and_drop() {
    let env = WipTestEnv::new();
    env.dirty("temp.txt", "throwaway");

    env.wip(&["save", "drop-me", "--force"]).assert().success();

    env.wip(&["drop", "drop-me"])
        .assert()
        .success()
        .stdout(contains("dropped"));

    // List should be empty now
    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("no WIP entries found"));
}

#[test]
fn test_save_force_overwrite() {
    let env = WipTestEnv::new();

    // First save
    env.dirty("v1.txt", "version 1");
    env.wip(&["save", "overwrite-me"]).assert().success();

    // Second save same name with --force
    env.dirty("v2.txt", "version 2");
    env.wip(&["save", "overwrite-me", "--force"])
        .assert()
        .success();
}

#[test]
fn test_save_duplicate_without_force() {
    let env = WipTestEnv::new();

    // First save
    env.dirty("a.txt", "first");
    env.wip(&["save", "dup-name"])
        .assert()
        .success()
        .stdout(contains("dup-name-01"));

    // Second save same name should auto-increment
    env.dirty("b.txt", "second");
    env.wip(&["save", "dup-name"])
        .assert()
        .success()
        .stdout(contains("dup-name-02"));
}

#[test]
fn test_save_with_task() {
    let env = WipTestEnv::new();
    env.dirty("task-file.txt", "task work");

    env.wip(&[
        "save",
        "task-test",
        "-m",
        "for ticket",
        "--task",
        "PROJ-42",
        "--force",
    ])
    .assert()
    .success();

    // Discard so show can fetch
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    let output = env.wip(&["show", "task-test"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("PROJ-42"), "should contain the task id");
}

#[test]
fn test_load_with_pop() {
    let env = WipTestEnv::new();
    env.dirty("pop-file.txt", "pop content");

    env.wip(&["save", "pop-test", "--force"]).assert().success();

    // Discard
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    // Load with --pop
    env.wip(&["load", "pop-test", "--pop"])
        .assert()
        .success()
        .stdout(contains("dropped"));

    assert_eq!(env.read_file("pop-file.txt"), "pop content");

    // Should be gone from list
    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("no WIP entries found"));
}

#[test]
fn test_save_auto_name() {
    let env = WipTestEnv::new();
    env.dirty("auto.txt", "auto-named");

    let branch = git(&env.local, &["rev-parse", "--abbrev-ref", "HEAD"]);

    // Save without a name — should auto-generate <branch>-01
    let output = env.wip(&["save"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    let expected = format!("{branch}-01");
    assert!(
        stdout.contains(&expected),
        "auto name should be {expected}, got: {stdout}"
    );
}

#[test]
fn test_save_clean_tree() {
    let env = WipTestEnv::new();
    // No dirty files

    env.wip(&["save", "clean-test"])
        .assert()
        .success()
        .stdout(contains("nothing to save"));

    // No ref should have been created
    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("no WIP entries found"));
}

#[test]
fn test_gc_expires_old() {
    let env = WipTestEnv::new();

    // Create a wip ref with a backdated commit (2 days ago) so gc reliably expires it.
    // We do this manually rather than through `wip save` to control the timestamp.
    env.dirty("gc-file.txt", "will expire");
    git(&env.local, &["add", "-A"]);
    let old_date = "2020-01-01T00:00:00+00:00";
    git_env(
        &env.local,
        &[
            "commit",
            "-m",
            "wip: gc test\n\n[wip-push]\nbranch=master\nfiles=1\nuntracked=1",
        ],
        &[
            ("GIT_AUTHOR_DATE", old_date),
            ("GIT_COMMITTER_DATE", old_date),
        ],
    );
    let sha = git(&env.local, &["rev-parse", "HEAD"]);
    let wip_ref = "refs/wip/test-user/gc-target";
    git(
        &env.local,
        &["push", "origin", &format!("{sha}:{wip_ref}"), "--force"],
    );
    // Reset back so the working tree is clean
    git(&env.local, &["reset", "HEAD~1", "--mixed"]);
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    // Verify it shows up in list
    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("gc-target"));

    // gc --dry-run with --expire 1d should identify it
    env.wip(&["gc", "--expire", "1d", "--dry-run"])
        .assert()
        .success()
        .stdout(contains("would drop"));

    // Actually delete
    env.wip(&["gc", "--expire", "1d"]).assert().success();

    // Should be gone
    env.wip(&["list"])
        .assert()
        .success()
        .stdout(contains("no WIP entries found"));
}

#[test]
fn test_load_with_conflict() {
    let env = WipTestEnv::new();

    // Save a change to README
    env.dirty("README", "remote version");
    env.wip(&["save", "conflict-test", "-m", "remote change", "--force"])
        .assert()
        .success();

    // Discard the local dirty state
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    // Make a different local change to the same file and commit it
    env.dirty("README", "local version");
    git(&env.local, &["add", "README"]);
    git(&env.local, &["commit", "-m", "local change"]);

    // Load should report conflicts
    env.wip(&["load", "conflict-test"])
        .assert()
        .success()
        .stdout(contains("conflicts"));
}

#[test]
fn test_load_with_theirs() {
    let env = WipTestEnv::new();

    // Save a change to README
    env.dirty("README", "incoming content");
    env.wip(&["save", "theirs-test", "-m", "theirs change", "--force"])
        .assert()
        .success();

    // Discard
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);

    // Commit a conflicting local change
    env.dirty("README", "local content");
    git(&env.local, &["add", "README"]);
    git(&env.local, &["commit", "-m", "local diverge"]);

    // Load with --theirs should resolve cleanly
    env.wip(&["load", "theirs-test", "--theirs"])
        .assert()
        .success()
        .stdout(contains("loaded"));

    // Incoming content should win
    assert_eq!(env.read_file("README"), "incoming content");
}

#[test]
fn test_load_dirty_tree_autostash() {
    let env = WipTestEnv::new();

    // Save something
    env.dirty("stuff.txt", "saved stuff");
    env.wip(&["save", "dirty-test", "--force"])
        .assert()
        .success();

    // Discard saved changes but leave the tree dirty with a different file
    git(&env.local, &["checkout", "--", "."]);
    git(&env.local, &["clean", "-fd"]);
    env.dirty("unrelated.txt", "uncommitted");

    // Load should succeed via autostash
    env.wip(&["load", "dirty-test"])
        .assert()
        .success()
        .stdout(contains("auto-stashed"));

    // Both loaded file and pre-existing dirty file should survive
    assert_eq!(env.read_file("stuff.txt"), "saved stuff");
    assert_eq!(env.read_file("unrelated.txt"), "uncommitted");
}

#[test]
fn test_save_with_stash() {
    let env = WipTestEnv::new();
    env.dirty("stash-me.txt", "stash content");
    env.dirty("also-stash.txt", "more content");

    // Save with --stash should clean the working tree
    env.wip(&["save", "stash-test", "--stash", "--force"])
        .assert()
        .success()
        .stdout(contains("stashed"));

    // Files should be gone from working tree
    assert!(
        !env.file_exists("stash-me.txt"),
        "stashed file should be removed"
    );
    assert!(
        !env.file_exists("also-stash.txt"),
        "stashed file should be removed"
    );

    // Load it back
    env.wip(&["load", "stash-test"])
        .assert()
        .success()
        .stdout(contains("loaded"));

    assert_eq!(env.read_file("stash-me.txt"), "stash content");
    assert_eq!(env.read_file("also-stash.txt"), "more content");
}

#[test]
fn test_bare_wip_defaults_to_save() {
    let env = WipTestEnv::new();
    env.dirty("bare.txt", "bare save");

    // Running `wip` with no subcommand should default to save
    env.wip(&[])
        .assert()
        .success()
        .stdout(contains("saved"))
        .stdout(contains("-01"));
}

#[test]
fn test_save_auto_increment() {
    let env = WipTestEnv::new();

    // First save (no name)
    env.dirty("inc1.txt", "first");
    let output = env.wip(&["save"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("-01"), "first save should be -01: {stdout}");

    // Second save (no name)
    env.dirty("inc2.txt", "second");
    let output = env.wip(&["save"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(
        stdout.contains("-02"),
        "second save should be -02: {stdout}"
    );

    // Both should appear in list
    let output = env.wip(&["list"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("-01"), "list should contain -01");
    assert!(stdout.contains("-02"), "list should contain -02");
}

#[test]
fn test_list_shows_metadata() {
    let env = WipTestEnv::new();
    env.dirty("meta.txt", "metadata test");

    env.wip(&[
        "save",
        "meta-test",
        "-m",
        "halfway through OAuth",
        "--task",
        "PROJ-42",
    ])
    .assert()
    .success();

    let output = env.wip(&["list"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(stdout.contains("meta-test"), "should contain the name");
    assert!(
        stdout.contains("halfway through OAuth"),
        "should contain the message"
    );
    assert!(stdout.contains("PROJ-42"), "should contain the task");
    assert!(
        stdout.contains("ago") || stdout.contains("just now"),
        "should contain relative time"
    );
}
