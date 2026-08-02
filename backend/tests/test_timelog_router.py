"""Tests for the time tracking endpoints."""

from datetime import date, timedelta
from pathlib import Path

from fastapi.testclient import TestClient


def _add_project(client: TestClient, tmp_path: Path, name: str = "Maths") -> str:
    todo = tmp_path / f"{name}.md"
    todo.write_text("- [ ] x\n")
    r = client.post(
        "/projects", json={"name": name, "file_path": str(todo), "type": "learning"}
    )
    assert r.status_code == 201, r.text
    return r.json()["id"]


def test_post_time_then_list(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    today = date.today().isoformat()

    r = client.post(f"/projects/{pid}/time", json={"minutes": 90, "topic": "Proba"})
    assert r.status_code == 201, r.text
    assert r.json()["date"] == today
    assert r.json()["minutes"] == 90

    assert client.get(f"/projects/{pid}/time").json() == [
        {"date": today, "minutes": 90}
    ]


def test_time_sums_entries_per_day(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    today = date.today()
    yesterday = (today - timedelta(days=1)).isoformat()

    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": today.isoformat()})
    client.post(f"/projects/{pid}/time", json={"minutes": 15, "date": today.isoformat()})
    client.post(f"/projects/{pid}/time", json={"minutes": 30, "date": yesterday})

    by_date = {d["date"]: d["minutes"] for d in client.get(f"/projects/{pid}/time").json()}
    assert by_date == {today.isoformat(): 75, yesterday: 30}


def test_time_respects_days_window(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    old = (date.today() - timedelta(days=200)).isoformat()
    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": old})
    client.post(f"/projects/{pid}/time", json={"minutes": 30})

    assert len(client.get(f"/projects/{pid}/time?days=120").json()) == 1
    assert len(client.get(f"/projects/{pid}/time?days=365").json()) == 2


def test_time_is_scoped_to_project(client: TestClient, tmp_path: Path):
    a = _add_project(client, tmp_path, "A")
    b = _add_project(client, tmp_path, "B")
    client.post(f"/projects/{a}/time", json={"minutes": 60})

    assert client.get(f"/projects/{b}/time").json() == []


def test_time_empty_window(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    assert client.get(f"/projects/{pid}/time").json() == []


def test_post_time_rejects_non_multiple_of_15(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    assert client.post(f"/projects/{pid}/time", json={"minutes": 20}).status_code == 422


def test_post_time_rejects_non_positive(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    assert client.post(f"/projects/{pid}/time", json={"minutes": 0}).status_code == 422
    assert client.post(f"/projects/{pid}/time", json={"minutes": -15}).status_code == 422


def test_time_unknown_project_404(client: TestClient):
    assert client.get("/projects/nope/time").status_code == 404
    assert client.post("/projects/nope/time", json={"minutes": 15}).status_code == 404
    assert client.get("/projects/nope/time/topics").status_code == 404


def test_topics_deduplicated_most_recent_first(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    today = date.today()
    days_ago = lambda n: (today - timedelta(days=n)).isoformat()  # noqa: E731

    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": days_ago(5), "topic": "Proba"})
    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": days_ago(3), "topic": "Stats"})
    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": days_ago(1), "topic": "Proba"})
    client.post(f"/projects/{pid}/time", json={"minutes": 60, "date": days_ago(0)})

    assert client.get(f"/projects/{pid}/time/topics").json() == ["Proba", "Stats"]


def test_blank_topic_is_stored_as_null(client: TestClient, tmp_path: Path):
    pid = _add_project(client, tmp_path)
    r = client.post(f"/projects/{pid}/time", json={"minutes": 15, "topic": "  "})
    assert r.json()["topic"] is None
    assert client.get(f"/projects/{pid}/time/topics").json() == []
