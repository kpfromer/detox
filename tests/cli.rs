use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

macro_rules! gen_folder {
    { $($path: expr => $value: expr),+ } => {
        {
            let folder = tempdir()?;

            $(
            {
                    let file_path = folder.path().join($path);
                    let mut file = std::fs::File::create(file_path)?;
                    writeln!(file, $value)?;
                };
            )*

            folder
        }
    };
}

// Useful for debuggin
#[allow(unused_macros)]
macro_rules! print_logs {
    { $logs: expr} => {
        {
            let output = $logs.get_output();
            println!("--------------STDOUT--------------");
            println!("{}", std::str::from_utf8(&output.stdout)?);
            println!("--------------STDERR--------------");
            println!("{}", std::str::from_utf8(&output.stderr)?);
        }
    };
}

#[test]
fn renames_files() -> TestResult {
    let folder = gen_folder! {
        "---bad__-file     name____-.mp4" => "bad file",
        "good-file.jpeg" => "good file"
    };
    let folder_path = folder.path();

    let mut cmd = Command::cargo_bin("detox")?;
    // --hidden needed for tests (due to tempfile)
    cmd.arg("--verbose").arg("--hidden").arg(folder_path);
    cmd.assert().success();

    let exists = predicate::path::exists();
    let does_not_exist = predicate::path::missing();

    assert_eq!(true, exists.eval(&folder_path.join("bad-file-name.mp4")));
    assert_eq!(
        true,
        does_not_exist.eval(&folder_path.join("---bad__-file-name____-.mp4"))
    );

    assert_eq!(true, exists.eval(&folder_path.join("good-file.jpeg")));

    Ok(())
}

#[test]
fn does_not_rename_with_dry_run() -> TestResult {
    let folder = gen_folder! {
        "---bad__-file     name____-.mp4" => "bad file",
        "good-file.jpeg" => "good file"
    };
    let folder_path = folder.path();

    let mut cmd = Command::cargo_bin("detox")?;
    cmd.arg("--verbose")
        .arg("--dry-run")
        .arg("--hidden")
        .arg(folder_path);
    cmd.assert().success();

    let exists = predicate::path::exists();
    let does_not_exist = predicate::path::missing();

    assert_eq!(
        true,
        exists.eval(&folder_path.join("---bad__-file     name____-.mp4"))
    );
    assert_eq!(true, exists.eval(&folder_path.join("good-file.jpeg")));
    assert_eq!(
        true,
        does_not_exist.eval(&folder_path.join("bad-file-name.mp4"))
    );

    Ok(())
}

#[test]
fn moves_overlapping_files() -> TestResult {
    let folder = gen_folder! {
        "overlapping-file.mp4" => "keep",
        "overlapping file.mp4" => "ahhh! I should be moved"
    };
    let folder_path = folder.path();
    let moved = tempdir()?;
    let moved_path = moved.path();

    let mut cmd = Command::cargo_bin("detox")?;
    // --hidden needed for tests (due to tempfile)
    cmd.arg("--verbose")
        .arg("--hidden")
        .arg("--move")
        .arg(moved_path)
        .arg(folder_path);

    cmd.assert().success();

    let exists = predicate::path::exists();
    let does_not_exist = predicate::path::missing();

    assert_eq!(true, exists.eval(&folder_path.join("overlapping-file.mp4")));
    let contents = std::fs::read_to_string(folder_path.join("overlapping-file.mp4"))?;
    assert_eq!(contents, "keep\n");

    // Moves file to moved folders
    assert_eq!(true, exists.eval(&moved_path.join("overlapping file.mp4")));
    assert_eq!(
        "ahhh! I should be moved\n",
        std::fs::read_to_string(moved_path.join("overlapping file.mp4"))?
    );
    assert_eq!(
        true,
        does_not_exist.eval(&folder_path.join("overlapping file.mp4"))
    );

    Ok(())
}

#[test]
fn does_not_move_overlapping_files_when_dry_run() -> TestResult {
    let folder = gen_folder! {
        "overlapping-file.mp4" => "keep",
        "overlapping file.mp4" => "ahhh! I should be moved"
    };
    let folder_path = folder.path();
    let moved = tempdir()?;
    let moved_path = moved.path();

    let mut cmd = Command::cargo_bin("detox")?;
    // --hidden needed for tests (due to tempfile)
    cmd.arg("--verbose")
        .arg("--hidden")
        .arg("--dry-run")
        .arg("--move")
        .arg(moved_path)
        .arg(folder_path);

    cmd.assert().success();

    let exists = predicate::path::exists();
    let does_not_exist = predicate::path::missing();

    assert_eq!(true, exists.eval(&folder_path.join("overlapping-file.mp4")));
    let contents = std::fs::read_to_string(folder_path.join("overlapping-file.mp4"))?;
    assert_eq!(contents, "keep\n");

    // Does not move file to moved folders
    assert_eq!(
        true,
        does_not_exist.eval(&moved_path.join("overlapping file.mp4"))
    );
    assert_eq!(true, exists.eval(&folder_path.join("overlapping file.mp4")));

    Ok(())
}

// #[test]
// fn does_not_rename_hidden_files() -> TestResult {
//     Ok(())
// }

// #[test]
// fn verbose() -> TestResult {
//     Ok(())
// }
