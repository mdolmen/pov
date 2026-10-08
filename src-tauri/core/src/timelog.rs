//! Time spent per project: TIME.md import and manually recorded entries.
//!
//! Unlike activity, this data has no git source of truth: it lives in SQLite.
//!
//! TIME.md is a markdown table (the header separator row is skipped):
//!
//! ```text
//! | DATE       | TIME | TOPIC      |
//! | ---------- | ---- | ---------- |
//! | 2026-01-31 | 2,00 | Proba      |
//! ```
//!
//! TIME is a number of hours with either a comma or a dot as decimal separator, rounded to the
//! nearest 15 minutes.

use std::path::Path;

use chrono::{Days, Local, NaiveDate, Weekday};
use rusqlite::Connection;
use serde::Serialize;

use crate::pyline::{read_text_strict, split_lines, strip};
use crate::{Error, Result, projects};

pub const STEP_MINUTES: i64 = 15;

/// One TIME.md row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeRow {
    pub date: String,
    pub minutes: i64,
    pub topic: Option<String>,
}

/// Minutes recorded on one day (`TimeDay` in the frontend).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeDay {
    pub date: String,
    pub minutes: i64,
}

/// A recorded time entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeEntry {
    pub id: i64,
    pub date: String,
    pub minutes: i64,
    pub topic: Option<String>,
}

/// Parses a date the way Python 3.11's `date.fromisoformat` does: `YYYY-MM-DD`, `YYYYMMDD`,
/// and the ISO week forms `YYYY-Www[-D]` / `YYYYWww[D]`.
pub fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    let b = s.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = s.get(range)?;
        part.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let year = i32::try_from(digits(0..4)?).ok().filter(|&y| y >= 1)?;
    let (rest, dashed) = match b.get(4) {
        Some(b'-') => (&s[5..], true),
        _ => (&s[4..], false),
    };

    if let Some(week) = rest.strip_prefix('W') {
        let (week, day) = match (week.len(), dashed) {
            (2, _) => (week, "1"),
            (4, true) if week.as_bytes()[2] == b'-' => (&week[..2], &week[3..]),
            (3, false) => (&week[..2], &week[2..]),
            _ => return None,
        };
        let week: u32 = week
            .parse()
            .ok()
            .filter(|_| week.bytes().all(|c| c.is_ascii_digit()))?;
        let weekday = match day {
            "1" => Weekday::Mon,
            "2" => Weekday::Tue,
            "3" => Weekday::Wed,
            "4" => Weekday::Thu,
            "5" => Weekday::Fri,
            "6" => Weekday::Sat,
            "7" => Weekday::Sun,
            _ => return None,
        };
        return NaiveDate::from_isoywd_opt(year, week, weekday);
    }

    let (month, day) = match (s.len(), dashed) {
        (10, true) if b[7] == b'-' => (digits(5..7)?, digits(8..10)?),
        (8, false) => (digits(4..6)?, digits(6..8)?),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, month, day)
}

fn cells(line: &str) -> Vec<&str> {
    let stripped = strip(line);
    if !stripped.starts_with('|') {
        return vec![];
    }
    stripped.trim_matches('|').split('|').map(strip).collect()
}

/// Parses a TIME.md file, one row per table data row in file order.
pub fn parse(path: &Path) -> Result<Vec<TimeRow>> {
    let text = read_text_strict(path)?;
    let error = |line: usize, msg: String| Error::TimeLogParse {
        path: path.to_owned(),
        line,
        msg,
    };

    let mut rows = Vec::new();
    for (i, line) in split_lines(&text).into_iter().enumerate() {
        let lineno = i + 1;
        let cells = cells(line);
        if cells.len() < 2 {
            continue;
        }
        let (raw_date, raw_hours) = (cells[0], cells[1]);
        if raw_date.chars().all(|c| c == '-' || c == ' ') || raw_date.to_uppercase() == "DATE" {
            continue;
        }
        let date = parse_iso_date(raw_date)
            .ok_or_else(|| error(lineno, format!("bad date '{raw_date}'")))?;
        let hours: f64 = raw_hours
            .replace(',', ".")
            .parse()
            .ok()
            .filter(|h: &f64| h.is_finite())
            .ok_or_else(|| error(lineno, format!("bad duration '{raw_hours}'")))?;
        // Python's round() rounds half to even.
        let minutes = (hours * 60.0 / STEP_MINUTES as f64).round_ties_even() as i64 * STEP_MINUTES;
        if minutes <= 0 {
            return Err(error(
                lineno,
                format!("non-positive duration '{raw_hours}'"),
            ));
        }
        let topic = cells
            .get(2)
            .map(|t| strip(t))
            .filter(|t| !t.is_empty())
            .map(String::from);
        rows.push(TimeRow {
            date: date.format("%Y-%m-%d").to_string(),
            minutes,
            topic,
        });
    }
    Ok(rows)
}

/// Inserts TIME.md rows for a project. Returns the number of rows inserted.
pub fn insert(conn: &Connection, project_id: &str, rows: &[TimeRow]) -> Result<usize> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        tx.execute(
            "INSERT INTO time_entries (project_id, date, minutes, topic) VALUES (?, ?, ?, ?)",
            (project_id, &row.date, row.minutes, &row.topic),
        )?;
    }
    tx.commit()?;
    Ok(rows.len())
}

/// Per-day minute totals for the last `days` days. Days with no recorded time are omitted.
pub fn list_days(conn: &Connection, project_id: &str, days: u64) -> Result<Vec<TimeDay>> {
    projects::file_and_type(conn, project_id)?;
    let today = Local::now().date_naive();
    let since = today
        .checked_sub_days(Days::new(days))
        .unwrap_or(NaiveDate::MIN);
    let mut stmt = conn.prepare(
        "SELECT date, SUM(minutes) AS minutes FROM time_entries \
         WHERE project_id = ? AND date >= ? GROUP BY date ORDER BY date",
    )?;
    let rows = stmt.query_map((project_id, since.format("%Y-%m-%d").to_string()), |row| {
        Ok(TimeDay {
            date: row.get(0)?,
            minutes: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Records time spent on a project.
///
/// `minutes` must be a positive multiple of 15. `date` is `YYYY-MM-DD` and defaults to today;
/// a blank topic is stored as none.
pub fn add_entry(
    conn: &Connection,
    project_id: &str,
    minutes: i64,
    date: Option<&str>,
    topic: Option<&str>,
) -> Result<TimeEntry> {
    if minutes <= 0 {
        return Err(Error::Invalid("minutes must be greater than 0".into()));
    }
    if minutes % STEP_MINUTES != 0 {
        return Err(Error::Invalid(format!(
            "minutes must be a multiple of {STEP_MINUTES}"
        )));
    }
    let date = match date {
        Some(d) => parse_iso_date(d).ok_or_else(|| Error::Invalid(format!("bad date '{d}'")))?,
        None => Local::now().date_naive(),
    }
    .format("%Y-%m-%d")
    .to_string();
    let topic = topic.map(strip).filter(|t| !t.is_empty()).map(String::from);

    projects::file_and_type(conn, project_id)?;
    conn.execute(
        "INSERT INTO time_entries (project_id, date, minutes, topic) VALUES (?, ?, ?, ?)",
        (project_id, &date, minutes, &topic),
    )?;
    Ok(TimeEntry {
        id: conn.last_insert_rowid(),
        date,
        minutes,
        topic,
    })
}

/// Topics used on a project, most recently used first.
pub fn topics(conn: &Connection, project_id: &str) -> Result<Vec<String>> {
    projects::file_and_type(conn, project_id)?;
    let mut stmt = conn.prepare(
        "SELECT topic FROM time_entries WHERE project_id = ? AND topic IS NOT NULL \
         GROUP BY topic ORDER BY MAX(date) DESC, MAX(id) DESC",
    )?;
    let topics = stmt.query_map([project_id], |row| row.get(0))?;
    Ok(topics.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{Paths, db};

    const TABLE: &str = "\
| DATE       | TIME | TOPIC          |
| ---------- | ---- | -------------- |
| 2026-01-31 | 2,00 | Proba          |
| 2026-01-31 | 1,50 | Statistics     |
| 2026-02-01 | 0,25 |                |
";

    fn parse_str(content: &str) -> Result<Vec<TimeRow>> {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("TIME.md");
        fs::write(&path, content).unwrap();
        parse(&path)
    }

    fn row(date: &str, minutes: i64, topic: Option<&str>) -> TimeRow {
        TimeRow {
            date: date.into(),
            minutes,
            topic: topic.map(String::from),
        }
    }

    /// A DB with projects `a` and `b`.
    fn conn() -> (tempfile::TempDir, Connection) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        db::init(&paths).unwrap();
        let conn = db::open(&paths).unwrap();
        conn.execute_batch(
            "INSERT INTO projects (id, name, file_path, type) VALUES
                 ('a', 'A', '/tmp/a.md', 'learning'), ('b', 'B', '/tmp/b.md', 'learning');",
        )
        .unwrap();
        (tmp, conn)
    }

    fn days_ago(n: u64) -> String {
        let day = Local::now()
            .date_naive()
            .checked_sub_days(Days::new(n))
            .unwrap();
        day.format("%Y-%m-%d").to_string()
    }

    #[test]
    fn parse_converts_hours_to_minutes() {
        assert_eq!(
            parse_str(TABLE).unwrap(),
            [
                row("2026-01-31", 120, Some("Proba")),
                row("2026-01-31", 90, Some("Statistics")),
                row("2026-02-01", 15, None),
            ]
        );
    }

    #[test]
    fn parse_accepts_dot_separator_and_rounds_to_15_minutes() {
        assert_eq!(
            parse_str("| 2026-01-31 | 1.75 | Proba |\n").unwrap()[0].minutes,
            105
        );
        assert_eq!(
            parse_str("| 2026-01-31 | 1,10 | Proba |\n").unwrap()[0].minutes,
            60
        );
    }

    #[test]
    fn parse_rejects_bad_date_and_duration() {
        let err = parse_str("| 31/01/2026 | 1,00 | Proba |\n").unwrap_err();
        assert!(err.to_string().contains(":1: bad date"), "{err}");
        let err = parse_str("| 2026-01-31 | two | Proba |\n").unwrap_err();
        assert!(err.to_string().contains("bad duration"), "{err}");
        let err = parse_str("| 2026-01-31 | nan | Proba |\n").unwrap_err();
        assert!(err.to_string().contains("bad duration"), "{err}");
    }

    #[test]
    fn iso_dates_follow_python() {
        let d = |s| parse_iso_date(s).map(|d| d.format("%Y-%m-%d").to_string());
        assert_eq!(d("2026-01-31").as_deref(), Some("2026-01-31"));
        assert_eq!(d("20260131").as_deref(), Some("2026-01-31"));
        assert_eq!(d("2026-W05-6").as_deref(), Some("2026-01-31"));
        assert_eq!(d("2026W056").as_deref(), Some("2026-01-31"));
        assert_eq!(d("2026-W05").as_deref(), Some("2026-01-26"));
        assert_eq!(d("2026W05").as_deref(), Some("2026-01-26"));
        for bad in [
            "2026-1-31",
            "2026-13-01",
            "0000-01-01",
            "2026-01-31 ",
            "",
            "2026/01/31",
        ] {
            assert_eq!(d(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn insert_stores_rows() {
        let (_tmp, conn) = conn();
        let rows = parse_str(TABLE).unwrap();
        assert_eq!(insert(&conn, "a", &rows).unwrap(), 3);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM time_entries", (), |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn add_then_list() {
        let (_tmp, conn) = conn();
        let entry = add_entry(&conn, "a", 90, None, Some("Proba")).unwrap();
        assert_eq!(
            (entry.date.as_str(), entry.minutes),
            (days_ago(0).as_str(), 90)
        );
        assert_eq!(
            list_days(&conn, "a", 120).unwrap(),
            [TimeDay {
                date: days_ago(0),
                minutes: 90
            }]
        );
    }

    #[test]
    fn list_sums_entries_per_day() {
        let (_tmp, conn) = conn();
        add_entry(&conn, "a", 60, Some(&days_ago(0)), None).unwrap();
        add_entry(&conn, "a", 15, Some(&days_ago(0)), None).unwrap();
        add_entry(&conn, "a", 30, Some(&days_ago(1)), None).unwrap();
        assert_eq!(
            list_days(&conn, "a", 120).unwrap(),
            [
                TimeDay {
                    date: days_ago(1),
                    minutes: 30
                },
                TimeDay {
                    date: days_ago(0),
                    minutes: 75
                },
            ]
        );
    }

    #[test]
    fn list_respects_window_and_project() {
        let (_tmp, conn) = conn();
        add_entry(&conn, "a", 60, Some(&days_ago(200)), None).unwrap();
        add_entry(&conn, "a", 30, None, None).unwrap();
        assert_eq!(list_days(&conn, "a", 120).unwrap().len(), 1);
        assert_eq!(list_days(&conn, "a", 365).unwrap().len(), 2);
        assert!(list_days(&conn, "b", 365).unwrap().is_empty());
    }

    #[test]
    fn add_rejects_bad_minutes() {
        let (_tmp, conn) = conn();
        for minutes in [20, 0, -15] {
            assert!(matches!(
                add_entry(&conn, "a", minutes, None, None),
                Err(Error::Invalid(_))
            ));
        }
    }

    #[test]
    fn unknown_project_is_not_found() {
        let (_tmp, conn) = conn();
        assert!(matches!(
            list_days(&conn, "nope", 120),
            Err(Error::NotFound("project"))
        ));
        assert!(matches!(
            add_entry(&conn, "nope", 15, None, None),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(
            topics(&conn, "nope"),
            Err(Error::NotFound("project"))
        ));
    }

    #[test]
    fn topics_are_deduplicated_most_recent_first() {
        let (_tmp, conn) = conn();
        add_entry(&conn, "a", 60, Some(&days_ago(5)), Some("Proba")).unwrap();
        add_entry(&conn, "a", 60, Some(&days_ago(3)), Some("Stats")).unwrap();
        add_entry(&conn, "a", 60, Some(&days_ago(1)), Some("Proba")).unwrap();
        add_entry(&conn, "a", 60, Some(&days_ago(0)), None).unwrap();
        assert_eq!(topics(&conn, "a").unwrap(), ["Proba", "Stats"]);
    }

    #[test]
    fn blank_topic_is_stored_as_none() {
        let (_tmp, conn) = conn();
        assert_eq!(
            add_entry(&conn, "a", 15, None, Some("  ")).unwrap().topic,
            None
        );
        assert!(topics(&conn, "a").unwrap().is_empty());
    }
}
