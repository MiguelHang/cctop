import json
import math
import mmap
import os
import subprocess
import sys
import time

CHUNK = 8 << 20
SLICE = 0.05


def spawn(child, links):
    env = dict(os.environ, SHOT_SPEC=json.dumps(child))
    argv = [child["name"], *child.get("args", [])]
    return subprocess.Popen(argv, executable=os.path.join(links, child["name"]), env=env, stdin=subprocess.DEVNULL)


def write_session(session):
    folder = os.path.join(session["config"], "sessions")
    os.makedirs(folder, exist_ok=True)
    with open(os.path.join(folder, f"{os.getpid()}.json"), "w") as file:
        json.dump({"pid": os.getpid(), "sessionId": session["id"], "cwd": session["cwd"]}, file)


def target(spec, elapsed):
    ramp = min(1.0, elapsed / spec.get("ramp", 1.0))
    memory = spec.get("mem", 0) + spec.get("grow", 0) * ramp
    wave = math.sin(elapsed * 2 * math.pi / spec.get("period", 30) + spec.get("phase", 0))
    duty = spec.get("cpu", 0) + spec.get("swing", 0) * wave
    return int(memory) << 20, max(0.0, min(1.0, duty))


def resize(chunks, size):
    while len(chunks) * CHUNK > size + CHUNK // 2:
        chunks.pop().close()
    while len(chunks) * CHUNK < size - CHUNK // 2:
        chunk = mmap.mmap(-1, CHUNK, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
        for offset in range(0, CHUNK, mmap.PAGESIZE):
            chunk[offset] = 1
        chunks.append(chunk)


def burn(duty):
    start = time.monotonic()
    while time.monotonic() - start < duty * SLICE:
        pass
    time.sleep(max(0.0, SLICE - (time.monotonic() - start)))


def run(spec):
    parent = os.getppid()
    until = float(os.environ["SHOT_UNTIL"])
    links = os.environ["SHOT_LINKS"]
    if "session" in spec:
        write_session(spec["session"])
    children = [spawn(child, links) for child in spec.get("children", [])]
    chunks = []
    start = time.monotonic()
    while time.time() < until and os.getppid() == parent:
        size, duty = target(spec, time.monotonic() - start)
        resize(chunks, size)
        burn(duty)
    for child in children:
        child.terminate()
    for child in children:
        child.wait()
    os._exit(0)


if "SHOT_SPEC" in os.environ:
    try:
        run(json.loads(os.environ["SHOT_SPEC"]))
    finally:
        os._exit(1)
