//! Activity: how recently a project was worked on, and per-day counts for the heatmap.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use chrono::{DateTime, TimeDelta, Utc};
use regex::Regex;
use rusqlite::Connection;
use serde::Serialize;

use crate::timelog::parse_iso_date;
use crate::{Paths, Result, git};

// Watcher and toggle commit messages: "activity: <project_id>.md".
static ACTIVITY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^activity:[\s\x1c-\x1f]+([0-9a-f-]+)\.md$").unwrap());

/// How recently a project was worked on, least recent first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    None,
    Older,
    ThisMonth,
    ThisWeek,
}

/// Commits on one day (`ActivityDay` in the frontend).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivityDay {
    pub date: String,
    pub count: u32,
}

/// Classifies a moment: this week (7 days), this month (30 days), older, or none.
pub fn classify(when: Option<DateTime<Utc>>) -> Level {
    let Some(when) = when else { return Level::None };
    let now = Utc::now();
    if when >= now - TimeDelta::days(7) {
        Level::ThisWeek
    } else if when >= now - TimeDelta::days(30) {
        Level::ThisMonth
    } else {
        Level::Older
    }
}

/// Classifies a `YYYY-MM-DD` date, as recorded time entries store it.
pub fn classify_day(iso_date: &str) -> Level {
    classify(parse_iso_date(iso_date).map(|d| d.and_time(Default::default()).and_utc()))
}

/// The level standing for the most recent activity.
pub fn most_recent(a: Level, b: Level) -> Level {
    a.max(b)
}

fn last_commit_date(paths: &Paths, file_path: &Path) -> Option<DateTime<Utc>> {
    let out = git::run(
        &paths.root,
        &[
            "log".as_ref(),
            "-1".as_ref(),
            "--format=%aI".as_ref(),
            "--".as_ref(),
            file_path.as_os_str(),
        ],
    )
    .ok()?;
    DateTime::parse_from_rfc3339(out.trim())
        .ok()
        .map(|d| d.to_utc())
}

fn mtime(file_path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(file_path)
        .and_then(|m| m.modified())
        .ok()
        .map(DateTime::from)
}

/// Activity level of a project file: last commit in the data repo when it has a hardlink,
/// falling back to the file's mtime.
pub fn level(paths: &Paths, file_path: &Path, has_hardlink: bool) -> Level {
    if has_hardlink && let Some(when) = last_commit_date(paths, file_path) {
        return classify(Some(when));
    }
    classify(mtime(file_path))
}

fn git_root(file_path: &Path) -> Option<PathBuf> {
    file_path
        .ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// The day of an ISO timestamp, in the timestamp's own offset.
fn day_of(timestamp: &str) -> Option<String> {
    let when = DateTime::parse_from_rfc3339(timestamp).ok()?;
    Some(when.date_naive().format("%Y-%m-%d").to_string())
}

fn git_log(repo: &Path, days: u64, format: &str) -> Vec<String> {
    let since = format!("--since={days} days ago");
    let format = format!("--pretty=format:{format}");
    match git::run(repo, &["log", &since, &format]) {
        Ok(out) => out.lines().map(String::from).collect(),
        Err(_) => vec![],
    }
}

/// Per-day activity of the projects of one type over the last `days` days.
///
/// Projects in a git repo count that repo's commits (a repo shared by several projects counts
/// once). Projects outside any repo count their `activity:` commits in the data repo. Days
/// without activity are omitted.
pub fn heatmap(
    conn: &Connection,
    paths: &Paths,
    project_type: &str,
    days: u64,
) -> Result<Vec<ActivityDay>> {
    let projects: Vec<(String, PathBuf)> = conn
        .prepare("SELECT id, file_path FROM projects WHERE type = ?")?
        .query_map([project_type], |row| {
            Ok((row.get(0)?, PathBuf::from(row.get::<_, String>(1)?)))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut roots = BTreeSet::new();
    let mut fallback_ids = BTreeSet::new();
    for (id, file_path) in projects {
        match git_root(&file_path) {
            Some(root) => roots.insert(root),
            None => fallback_ids.insert(id),
        };
    }

    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for root in &roots {
        for line in git_log(root, days, "%aI") {
            if let Some(day) = day_of(line.trim()) {
                *counts.entry(day).or_default() += 1;
            }
        }
    }
    if !fallback_ids.is_empty() {
        for line in git_log(&paths.root, days, "%aI%x09%s") {
            let Some((timestamp, subject)) = line.split_once('\t') else {
                continue;
            };
            let Some(m) = ACTIVITY_RE.captures(subject) else {
                continue;
            };
            if fallback_ids.contains(&m[1])
                && let Some(day) = day_of(timestamp)
            {
                *counts.entry(day).or_default() += 1;
            }
        }
    }

    Ok(counts
        .into_iter()
        .map(|(date, count)| ActivityDay { date, count })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;
    use crate::{db, storage};

    fn days_ago(n: i64) -> DateTime<Utc> {
        Utc::now() - TimeDelta::days(n)
    }

    fn commit_at(repo: &Path, when: DateTime<Utc>, message: &str) {
        let iso = when.to_rfc3339();
        let status = Command::new("git")
            .args(["-C".as_ref(), repo.as_os_str()])
            .args(["commit", "--allow-empty", "-m", message])
            .envs([
                ("GIT_AUTHOR_DATE", iso.as_str()),
                ("GIT_COMMITTER_DATE", &iso),
            ])
            .envs([("GIT_AUTHOR_NAME", "t"), ("GIT_AUTHOR_EMAIL", "t@t")])
            .envs([("GIT_COMMITTER_NAME", "t"), ("GIT_COMMITTER_EMAIL", "t@t")])
            .output()
            .unwrap()
            .status;
        assert!(status.success());
    }

    struct Env {
        tmp: tempfile::TempDir,
        paths: Paths,
        conn: Connection,
    }

    impl Env {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let paths = Paths::new(tmp.path().join("pov"));
            storage::init(&paths).unwrap();
            db::init(&paths).unwrap();
            let conn = db::open(&paths).unwrap();
            Self { tmp, paths, conn }
        }

        /// A directory under the temp dir, as a git repo when `git` is set.
        fn dir(&self, name: &str, git: bool) -> PathBuf {
            let dir = self.tmp.path().join(name);
            fs::create_dir_all(&dir).unwrap();
            if git {
                git::run(&dir, &["init"]).unwrap();
            }
            dir
        }

        fn add(&self, id: &str, dir: &Path, project_type: &str) {
            let todo = dir.join("TODO.md");
            fs::write(&todo, "- [ ] x\n").unwrap();
            self.conn
                .execute(
                    "INSERT INTO projects (id, name, file_path, type) VALUES (?, ?, ?, ?)",
                    [id, id, todo.to_str().unwrap(), project_type],
                )
                .unwrap();
        }

        fn total(&self, project_type: &str, days: u64) -> u32 {
            let days = heatmap(&self.conn, &self.paths, project_type, days).unwrap();
            days.iter().map(|d| d.count).sum()
        }
    }

    #[test]
    fn classify_by_age() {
        assert_eq!(classify(Some(days_ago(3))), Level::ThisWeek);
        assert_eq!(classify(Some(days_ago(15))), Level::ThisMonth);
        assert_eq!(classify(Some(days_ago(60))), Level::Older);
        assert_eq!(classify(None), Level::None);
        assert_eq!(classify_day("not a date"), Level::None);
        assert_eq!(
            most_recent(Level::Older, Level::ThisMonth),
            Level::ThisMonth
        );
    }

    #[test]
    fn level_uses_git_when_hardlinked() {
        let env = Env::new();
        let md = env.tmp.path().join("test.md");
        fs::write(&md, "- [ ] task\n").unwrap();
        let dst = env.paths.projects.join("test.md");
        fs::hard_link(&md, &dst).unwrap();
        git::add_commit(&env.paths.root, &dst, "add test").unwrap();

        assert_eq!(level(&env.paths, &dst, true), Level::ThisWeek);
    }

    #[test]
    fn level_falls_back_to_mtime() {
        let env = Env::new();
        let md = env.tmp.path().join("untracked.md");
        fs::write(&md, "- [ ] task\n").unwrap();

        assert_eq!(level(&env.paths, &md, false), Level::ThisWeek);
        // Not in the data repo: git log prints nothing.
        assert_eq!(level(&env.paths, &md, true), Level::ThisWeek);
    }

    #[test]
    fn heatmap_filters_by_type() {
        let env = Env::new();
        let proj = env.dir("proj", true);
        let learn = env.dir("learn", true);
        env.add("p", &proj, "project");
        env.add("l", &learn, "learning");
        commit_at(&proj, days_ago(0), "work");
        commit_at(&learn, days_ago(0), "work");
        commit_at(&learn, days_ago(2), "work");

        assert_eq!(env.total("project", 120), 1);
        assert_eq!(env.total("learning", 120), 2);
    }

    #[test]
    fn heatmap_buckets_by_day() {
        let env = Env::new();
        let repo = env.dir("proj", true);
        env.add("p", &repo, "project");
        let base = Utc::now()
            .date_naive()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_utc();
        commit_at(&repo, base, "work");
        commit_at(&repo, base + TimeDelta::hours(2), "work");
        commit_at(&repo, base - TimeDelta::days(1), "work");

        let day = |d: DateTime<Utc>| d.format("%Y-%m-%d").to_string();
        assert_eq!(
            heatmap(&env.conn, &env.paths, "project", 120).unwrap(),
            [
                ActivityDay {
                    date: day(base - TimeDelta::days(1)),
                    count: 1
                },
                ActivityDay {
                    date: day(base),
                    count: 2
                },
            ]
        );
    }

    #[test]
    fn heatmap_respects_days_window() {
        let env = Env::new();
        let repo = env.dir("proj", true);
        env.add("p", &repo, "project");
        commit_at(&repo, days_ago(200), "work");
        commit_at(&repo, days_ago(0), "work");

        assert_eq!(env.total("project", 120), 1);
        assert_eq!(env.total("project", 365), 2);
    }

    #[test]
    fn heatmap_counts_a_shared_repo_once() {
        let env = Env::new();
        let repo = env.dir("mono", true);
        env.add("a", &repo, "project");
        env.add("b", &env.dir("mono/sub", false), "project");
        commit_at(&repo, days_ago(0), "work");

        assert_eq!(env.total("project", 120), 1);
    }

    #[test]
    fn heatmap_without_projects_is_empty() {
        let env = Env::new();
        assert!(
            heatmap(&env.conn, &env.paths, "project", 120)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn heatmap_falls_back_to_data_repo_for_this_project_only() {
        let env = Env::new();
        let id = "11111111-1111-1111-1111-111111111111";
        env.add(id, &env.dir("plain", false), "project");
        commit_at(&env.paths.root, days_ago(0), &format!("activity: {id}.md"));
        commit_at(
            &env.paths.root,
            days_ago(0),
            "activity: 00000000-0000-0000-0000-000000000000.md",
        );

        assert_eq!(env.total("project", 120), 1);
    }
}
