import asyncio
import json
import socket
import sys

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from pov.db import init_db
from pov.routers import activity, projects, tasks, timelog
from pov.storage import init_storage
from pov.watcher import start_watcher

app = FastAPI(title="pov")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(projects.router)
app.include_router(tasks.router)
app.include_router(activity.router)
app.include_router(timelog.router)


@app.get("/health")
async def health() -> dict:
    return {"status": "ok"}


def pick_free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


async def startup() -> None:
    await init_storage()
    await init_db()


if __name__ == "__main__":
    port = pick_free_port()
    # Tauri reads this line from stdout to learn the port.
    print(json.dumps({"port": port}), flush=True)

    asyncio.run(startup())
    observer = start_watcher()
    try:
        uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")
    finally:
        observer.stop()
        observer.join()
