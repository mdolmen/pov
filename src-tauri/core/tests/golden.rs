//! Compares pov-core with outputs recorded from the Python backend (`backend/dev/golden.py`).

use std::fs;
use std::path::{Path, PathBuf};

use pov_core::Error;
use pov_core::tasks::{self, Item};
use pov_core::timelog;
use serde_json::{Value, json};

fn goldens(kind: &str) -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(kind);
    let mut goldens: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            (
                name,
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap(),
            )
        })
        .collect();
    goldens.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!goldens.is_empty());
    goldens
}

fn input(golden: &Value) -> Vec<u8> {
    match golden.get("input") {
        Some(text) => text.as_str().unwrap().as_bytes().to_vec(),
        None => golden["input_bytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| u8::try_from(b.as_u64().unwrap()).unwrap())
            .collect(),
    }
}

fn item_json(item: &Item) -> Value {
    match item {
        Item::Heading { text, level } => json!({"kind": "heading", "text": text, "level": level}),
        Item::Task(t) => json!({
            "kind": "task",
            "hash": t.hash,
            "text": t.text,
            "checked": t.checked,
            "line_number": t.line_number,
            "is_done": t.is_done(),
            "subtasks": t.subtasks,
        }),
    }
}

fn write_input(dir: &Path, golden: &Value) -> PathBuf {
    let path = dir.join("TODO.md");
    fs::write(&path, input(golden)).unwrap();
    path
}

#[test]
fn tasks_match_python() {
    let tmp = tempfile::tempdir().unwrap();
    for (name, golden) in goldens("tasks") {
        let path = write_input(tmp.path(), &golden);

        let items: Vec<Value> = tasks::parse_items(&path).iter().map(item_json).collect();
        assert_eq!(Value::from(items), golden["items"], "{name}: items");
        assert_eq!(
            json!(tasks::count_tasks(&path)),
            golden["count_tasks"],
            "{name}: count"
        );

        for (hash, expected) in golden["toggles"].as_object().unwrap() {
            let path = write_input(tmp.path(), &golden);
            assert!(
                tasks::toggle_cascade(&path, hash).unwrap(),
                "{name}: toggle {hash}"
            );
            let actual = fs::read_to_string(&path).unwrap();
            assert_eq!(actual, expected.as_str().unwrap(), "{name}: toggle {hash}");
        }
    }
}

#[test]
fn time_log_matches_python() {
    let tmp = tempfile::tempdir().unwrap();
    for (name, golden) in goldens("time") {
        let path = tmp.path().join("TIME.md");
        fs::write(&path, input(&golden)).unwrap();

        match timelog::parse(&path) {
            Ok(rows) => assert_eq!(json!(rows), golden["rows"], "{name}: rows"),
            Err(Error::TimeLogParse { line, .. }) => {
                assert_eq!(json!(line), golden["error_line"], "{name}: error line")
            }
            Err(e) => panic!("{name}: {e}"),
        }
    }
}
