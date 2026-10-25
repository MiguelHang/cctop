#!/usr/bin/env bash
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
binary="$repo/target/release/cctop"
warmup="${SHOTS_WARMUP:-180}"
cols=120
rows=34
clean_env=(env -i PATH=/usr/bin:/bin LANG=C.UTF-8)
tmux=("${clean_env[@]}" tmux -L shots)

if "${tmux[@]}" has-session 2>/dev/null; then
    echo "a tmux server named shots is already running" >&2
    exit 1
fi

work="$(mktemp -d)"
scene_pid=""
cleanup() {
    "${tmux[@]}" kill-server 2>/dev/null || true
    rm -f "/tmp/tmux-$(id -u)/shots"
    if [ -n "$scene_pid" ]; then
        kill -TERM -- "-$scene_pid" 2>/dev/null || true
    fi
    rm -rf "$work"
}
trap cleanup EXIT INT TERM

(cd "$repo" && cargo build --release --locked)

mkdir -p "$work/user" "$work/config/cctop"
printf 'interval = "100ms"\n' >"$work/config/cctop/config.toml"
cat >"$work/cornercase" <<SCRIPT
#!/bin/sh
[ "\$1" = status ] && exec cat "$work/status.json"
exit 0
SCRIPT
chmod +x "$work/cornercase"

setsid "${clean_env[@]}" python3 "$here/scene.py" "$work" "$((warmup + 60))" </dev/null &
scene_pid=$!

until compgen -G "$work/claude/sessions/*.json" >/dev/null; do
    kill -0 "$scene_pid" 2>/dev/null || { echo "the scene stopped" >&2; exit 1; }
    sleep 0.2
done

"${tmux[@]}" -f /dev/null new-session -d -x "$cols" -y "$rows" \
    "${clean_env[@]}" TERM=tmux-256color HOME="$work/user" XDG_CONFIG_HOME="$work/config" \
    CORNERCASE_BIN="$work/cornercase" CORNERCASE_CONTEXT=button CORNERCASE_PANE=1 \
    taskset -c 0-3 "$binary"
"${tmux[@]}" set-option -g status off

keys() {
    "${tmux[@]}" send-keys "$@"
    sleep 0.5
}

shot() {
    "${tmux[@]}" capture-pane -e -p >"$work/$1.ans"
    if [ -n "${SHOTS_KEEP:-}" ]; then
        cp "$work/$1.ans" "$SHOTS_KEEP/"
    fi
    python3 "$here/ansi2svg.py" "$cols" "$rows" <"$work/$1.ans" >"$here/$1.svg"
}

sleep "$warmup"
keys Right Down Right Down Right Down Right
keys Down Down Down Right Down Right Down Right Down Right
keys Down Down Right Down Right
keys Up Up Up Up Up Up Up Up Up Up Up Up
shot processes
keys Tab
sleep 2
shot context
keys q
