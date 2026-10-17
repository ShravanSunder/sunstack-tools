"""Controlled remote replies; the executable's real HTTP adapter stays in use."""

import json
import threading
import types
import typing as t
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from queue import Empty, Queue

from scenario_runtime import (
    JSON_OBJECT,
    JsonObject,
    as_array,
    as_object,
    object_at,
    write_object,
)


@dataclass(kw_only=True)
class PendingRequest:
    body: JsonObject
    raw: bytes
    responses: Queue[tuple[int, JsonObject]] = field(default_factory=Queue)
    response_count: int = 0

    def respond(self, status: int, body: JsonObject) -> None:
        assert self.response_count == 0, "A controlled request gets one response"
        self.response_count += 1
        self.responses.put((status, body))

    def answer(
        self, probability: float = 0.9, *, choice: str = "review_handback"
    ) -> None:
        self.respond(200, response_body(self.body, probability, choice))


def response_body(body: JsonObject, probability: float, choice: str) -> JsonObject:
    answers: JsonObject = {}
    for name, value in object_at(body, "questions").items():
        question = as_object(value)
        match question["type"]:
            case "noul":
                answer: JsonObject = {"type": "noul", "noul": probability}
            case "choice":
                criteria = object_at(question, "criteria")
                label = (
                    choice
                    if choice in criteria
                    else "none"
                    if "none" in criteria
                    else next(iter(criteria))
                )
                probabilities: JsonObject = {
                    key: 1.0 if key == label else 0.0 for key in criteria
                }
                answer = {
                    "type": "choice",
                    "choice": label,
                    "confidence": 1.0,
                    "probabilities": probabilities,
                }
            case "score":
                criteria = as_array(question["criteria"])
                last = len(criteria) - 1
                legend: JsonObject = {
                    str(index): item for index, item in enumerate(criteria)
                }
                probabilities = {
                    str(index): 1.0 if index == last else 0.0
                    for index in range(len(criteria))
                }
                answer = {
                    "type": "score",
                    "score": float(last),
                    "confidence": 1.0,
                    "legend": legend,
                    "probabilities": probabilities,
                }
            case _:
                raise AssertionError("Unrecognized controlled question primitive")
        answers[name] = answer
    return {
        "model": "typesafe/controlled-usage-scenario",
        "answers": answers,
        "usage": {"input_tokens": 20, "output_tokens": 5, "cost": 0.000001},
    }


class ControlledServer:
    def __init__(self, directory: Path) -> None:
        directory.mkdir(parents=True, exist_ok=True)
        self.pending: Queue[PendingRequest] = Queue()
        self.received: list[PendingRequest] = []
        self.lock = threading.Lock()
        owner = self

        class RequestHandler(BaseHTTPRequestHandler):
            def do_POST(self) -> None:
                size = int(self.headers["Content-Length"])
                assert 0 < size <= 2_000_000
                raw = self.rfile.read(size)
                body = JSON_OBJECT.validate_json(raw)
                pending = PendingRequest(body=body, raw=raw)
                with owner.lock:
                    owner.received.append(pending)
                    write_object(
                        directory / f"http-{len(owner.received):02d}.json", body
                    )
                    (
                        directory / f"http-{len(owner.received):02d}.raw.json"
                    ).write_bytes(raw)
                owner.pending.put(pending)
                try:
                    status, response = pending.responses.get(timeout=10)
                except Empty:
                    status, response = (
                        503,
                        {"error": "controlled response not supplied"},
                    )
                encoded = json.dumps(response).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(encoded)))
                self.end_headers()
                try:
                    self.wfile.write(encoded)
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def log_message(self, format: str, *args: t.Any) -> None:
                return

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), RequestHandler)
        self.server.daemon_threads = False
        self.thread = threading.Thread(target=self.server.serve_forever)
        self.thread.start()
        self.endpoint = f"http://127.0.0.1:{self.server.server_port}/systemone"

    def next(self) -> PendingRequest:
        return self.pending.get(timeout=8)

    def __enter__(self) -> t.Self:
        return self

    def __exit__(
        self,
        exception_type: type[BaseException] | None,
        exception: BaseException | None,
        traceback: types.TracebackType | None,
    ) -> None:
        for pending in self.received:
            if pending.response_count == 0:
                pending.respond(503, {"error": "owned control closed"})
        self.server.shutdown()
        self.thread.join(timeout=5)
        self.server.server_close()
        assert not self.thread.is_alive()

    @property
    def count(self) -> int:
        return len(self.received)

    def bodies(self) -> list[JsonObject]:
        return [pending.body for pending in self.received]
