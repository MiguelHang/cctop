import json
import os
import subprocess
import sys
import time

BOOT = "31337-1761390000000000000"
SESSION = "5e55a0b0-0000-4000-8000-00000000c0de"
NAMES = ["zsh", "claude", "node", "npm", "cargo", "rustc", "rust-analyzer", "nvim", "checkout-api"]


def proc(name, *args, mem=12, cpu=0.0, swing=0.0, period=30, phase=0.0, grow=0, ramp=1.0, children=(), **extra):
    spec = dict(name=name, args=list(args), mem=mem, cpu=cpu, swing=swing, period=period, phase=phase)
    spec.update(grow=grow, ramp=ramp, children=list(children), **extra)
    return spec


def shell(*children):
    return proc("zsh", mem=6, children=children)


def claude(mem, cpu=0.02, swing=0.0, children=(), **extra):
    return proc("claude", mem=mem, cpu=cpu, swing=swing, period=20, children=children, **extra)


def pane(id, program, root, agent=None, model=None, status=None, context=None):
    return dict(id=id, program=program, root=root, agent=agent, model=model, status=status, context=context)


def scene(config):
    session = dict(config=config, id=SESSION, cwd="/code/shop")
    build = proc("cargo", "test", "--locked", mem=110, cpu=0.08, children=[
        proc("rustc", "src/lib.rs", "--test",
             mem=260, grow=240, ramp=150, cpu=0.85, swing=0.15, period=11)])
    shop_claude = claude(360, cpu=0.12, swing=0.1, session=session,
                         children=[proc("node", "docs-mcp/index.js", mem=85, cpu=0.01), build])
    editor = proc("nvim", "src/cart.rs", mem=38, cpu=0.01, children=[
        proc("rust-analyzer", mem=520, grow=180, ramp=170, cpu=0.35, swing=0.35, period=40, phase=1.0)])
    return [
        ("shop", "/code/shop", None, [
            ("main", "main", False, [
                ("claude", [pane(1, "claude", shell(shop_claude), "claude", "opus", "working", 62)]),
                ("dev", [pane(2, "npm", shell(proc("npm", "run", "dev", mem=55, children=[
                    proc("node", "vite", "--port", "5173", mem=190, cpu=0.04, swing=0.04, period=7)])))]),
            ]),
            ("checkout-flow", "feat/checkout-flow", True, [
                ("claude", [pane(3, "claude", shell(claude(280)), "claude", "sonnet", "idle", 34),
                            pane(4, "nvim", shell(editor))]),
            ]),
        ]),
        ("checkout", "/code/checkout", 1, [
            ("main", "main", False, [
                ("api", [pane(5, "cargo", shell(proc("cargo", "run", "--release", mem=70, children=[
                    proc("checkout-api", mem=140, grow=40, ramp=90, cpu=0.45, swing=0.4, period=23, phase=2.0)])))]),
                ("claude", [pane(6, "claude", shell(claude(230)), "claude", "haiku", "idle", 18)]),
            ]),
        ]),
        ("search-bar", "/code/search-bar", 1, [
            ("main", "main", False, [
                ("claude", [pane(7, "claude", shell(claude(320, cpu=0.04)), "claude", "opus", "waiting", 81)]),
                ("tests", [pane(8, "node", shell(proc("node", "jest", "--watch", mem=170, cpu=0.15, swing=0.15,
                                                      period=17, phase=4.0)))]),
            ]),
        ]),
    ]


def status(projects):
    ids = {"workspace": 0, "tab": 0}
    out = []
    for number, (name, path, group, workspaces) in enumerate(projects, start=1):
        out.append(dict(id=number, name=name, path=path, group=group, active=number == 1,
                        workspaces=[workspace(ids, path, number == 1, *w) for w in workspaces]))
    groups = [dict(id=1, name="work", collapsed=False, colour=3, icon="")]
    return dict(boot=BOOT, version="0.12.187", groups=groups, projects=out)


def workspace(ids, project_path, first, name, branch, worktree, tabs):
    ids["workspace"] += 1
    path = f"/code/worktrees/{os.path.basename(project_path)}/{name}" if worktree else project_path
    return dict(id=ids["workspace"], name=name, branch=branch, path=path, worktree=worktree, active=first,
                tabs=[tab(ids, path, index == 0, *t) for index, t in enumerate(tabs)])


def tab(ids, path, first, name, panes):
    ids["tab"] += 1
    rows = [dict(id=p["id"], active=index == 0, agent=p["agent"], model=p["model"], status=p["status"],
                 context=p["context"], subagents=0, parent=None, program=p["program"], path=path)
            for index, p in enumerate(panes)]
    return dict(id=ids["tab"], name=name, active=first, panes=rows)


def panes(projects):
    for _, _, _, workspaces in projects:
        for _, _, _, tabs in workspaces:
            for _, rows in tabs:
                yield from rows


def links(folder):
    os.makedirs(folder, exist_ok=True)
    python = os.path.realpath(sys.executable)
    for name in NAMES:
        os.symlink(python, os.path.join(folder, name))


def start(root, links_dir, worker, until, config):
    env = dict(PATH="/usr/bin:/bin", CLAUDE_CONFIG_DIR=config, PYTHONPATH=worker, PYTHONDONTWRITEBYTECODE="1",
               SHOT_LINKS=links_dir, SHOT_UNTIL=str(until), CORNERCASE_SERVER=BOOT, CORNERCASE_PANE=str(root["id"]),
               SHOT_SPEC=json.dumps(root["root"]))
    executable = os.path.join(links_dir, root["root"]["name"])
    return subprocess.Popen([root["root"]["name"]], executable=executable, env=env, stdin=subprocess.DEVNULL)


def main(folder, seconds):
    config = os.path.join(folder, "claude")
    projects = scene(config)
    with open(os.path.join(folder, "status.json"), "w") as file:
        json.dump(status(projects), file, indent=1)
    transcript = os.path.join(config, "projects", "-code-shop")
    os.makedirs(transcript, exist_ok=True)
    subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), "transcript.py"),
                    os.path.join(transcript, f"{SESSION}.jsonl"), SESSION], check=True)
    links(os.path.join(folder, "bin"))
    worker = os.path.join(os.path.dirname(os.path.abspath(__file__)), "worker")
    until = time.time() + seconds
    roots = [start(p, os.path.join(folder, "bin"), worker, until, config) for p in panes(projects)]
    for root in roots:
        root.wait()


if __name__ == "__main__":
    main(sys.argv[1], float(sys.argv[2]))
