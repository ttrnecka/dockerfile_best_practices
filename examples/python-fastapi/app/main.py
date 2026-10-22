from fastapi import FastAPI
from fastapi.responses import HTMLResponse

app = FastAPI()


@app.get("/", response_class=HTMLResponse)
async def hello_world() -> str:
    return "<!doctype html><html><body><h1>Hello World</h1></body></html>"


@app.get("/healthz")
async def healthz() -> dict[str, str]:
    return {"status": "ok"}