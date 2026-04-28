import asyncio
import json
import socket
import sys

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from pov.db import init_db
from pov.storage import init_storage

app = FastAPI(title="pov")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)


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
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="warning")
