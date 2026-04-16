use std::process::Command;

fn fig2r_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fig2r"))
}

#[test]
fn test_convert_minimal_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let output = fig2r_bin()
        .args(["convert", "tests/fixtures/minimal.ir.json", "-o"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let card_path = dir.path().join("Card/Card.tsx");
    assert!(card_path.exists(), "Card.tsx should exist");
    let content = std::fs::read_to_string(&card_path).unwrap();
    assert!(content.contains("export function Card"));
    assert!(content.contains("Card Title"));
    assert!(content.contains("flex flex-col"));
    assert!(content.contains("p-[16px]"));
    assert!(content.contains("rounded-[8px]"));
    assert!(content.contains("shadow-["));

    let index_path = dir.path().join("Card/index.ts");
    assert!(index_path.exists(), "index.ts should exist");

    let theme_path = dir.path().join("theme/tailwind.extend.js");
    assert!(theme_path.exists(), "tailwind.extend.js should exist");
    let theme_content = std::fs::read_to_string(&theme_path).unwrap();
    assert!(theme_content.contains("primary"));
    assert!(theme_content.contains("#3B82F6"));
}

#[test]
fn test_validate_command() {
    let output = fig2r_bin()
        .args(["validate", "tests/fixtures/minimal.ir.json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[OK]"));
}

#[test]
fn test_validate_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let bad_file = dir.path().join("bad.json");
    std::fs::write(&bad_file, "not json").unwrap();
    let output = fig2r_bin()
        .args(["validate"])
        .arg(&bad_file)
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn test_convert_flat_flag() {
    let dir = tempfile::tempdir().unwrap();
    let output = fig2r_bin()
        .args(["convert", "tests/fixtures/minimal.ir.json", "-o"])
        .arg(dir.path())
        .arg("--flat")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(dir.path().join("Card.tsx").exists());
    assert!(!dir.path().join("Card/Card.tsx").exists());
}

#[test]
fn test_convert_no_theme() {
    let dir = tempfile::tempdir().unwrap();
    let output = fig2r_bin()
        .args(["convert", "tests/fixtures/minimal.ir.json", "-o"])
        .arg(dir.path())
        .arg("--no-theme")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!dir.path().join("theme/tailwind.extend.js").exists());
}

#[test]
fn test_convert_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let json = std::fs::read_to_string("tests/fixtures/minimal.ir.json").unwrap();
    let output = fig2r_bin()
        .args(["convert", "-o"])
        .arg(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(json.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("Card/Card.tsx").exists());
}

#[test]
fn test_convert_public_dir_writes_assets_outside_output() {
    let output_dir = tempfile::tempdir().unwrap();
    let public_dir = tempfile::tempdir().unwrap();

    let output = fig2r_bin()
        .args(["convert", "tests/fixtures/image.ir.json", "-o"])
        .arg(output_dir.path())
        .arg("--public-dir")
        .arg(public_dir.path())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_dir.path().join("Gallery/Gallery.tsx").exists());
    assert!(!output_dir.path().join("assets/hero-image.png").exists());
    assert!(public_dir.path().join("assets/hero-image.png").exists());
}
