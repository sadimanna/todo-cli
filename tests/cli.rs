use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;
use tempfile::tempdir;

#[test]
fn cli_add_list_done_delete() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("tasks.db");

    cargo_bin_cmd!("todo")
        .env("TODO_DB_PATH", &db_path)
        .args(["add", "Test task"])
        .assert()
        .success();

    cargo_bin_cmd!("todo")
        .env("TODO_DB_PATH", &db_path)
        .args(["list"])
        .assert()
        .success()
        .stdout(contains("Test task"));

    cargo_bin_cmd!("todo")
        .env("TODO_DB_PATH", &db_path)
        .args(["done", "1"])
        .assert()
        .success();

    cargo_bin_cmd!("todo")
        .env("TODO_DB_PATH", &db_path)
        .args(["delete", "1"])
        .assert()
        .success();
}
