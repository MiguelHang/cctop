import re
import sys
from html import escape

W, H, PAD, FONT, GAP = 9.0, 19.0, 14.0, 15, 1.5
BG, FG = "#0d1117", "#c9d1d9"
ANSI = ["#484f58", "#ff7b72", "#3fb950", "#d29922", "#58a6ff", "#bc8cff", "#39c5cf", "#b1bac4",
        "#6e7681", "#ffa198", "#56d364", "#e3b341", "#79c0ff", "#d2a8ff", "#56d4dd", "#f0f6fc"]
SGR = re.compile(r"\x1b\[([0-9;]*)m")
LINES = {"─": "lr", "│": "ud", "├": "udr", "┤": "udl", "┬": "lrd", "┴": "lru", "┼": "lrud",
         "┌": "rd", "┐": "ld", "└": "ru", "┘": "lu", "╭": "rd", "╮": "ld", "╰": "ru", "╯": "lu"}
ROUND = set("╭╮╰╯")
SHADES = {"░": 0.3, "▒": 0.55, "▓": 0.8, "█": 1.0}
BRAILLE = [(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (0, 3), (1, 3)]
RESETS = {22: (1, 2), 27: (7,)}


def palette(n):
    if n < 16:
        return ANSI[n]
    if n < 232:
        n -= 16
        levels = [0 if v == 0 else 55 + v * 40 for v in (n // 36, n // 6 % 6, n % 6)]
        return "#{:02x}{:02x}{:02x}".format(*levels)
    grey = 8 + (n - 232) * 10
    return f"#{grey:02x}{grey:02x}{grey:02x}"


def extended(codes):
    if codes.pop(0) == 5:
        return palette(codes.pop(0))
    red, green, blue = codes[:3]
    del codes[:3]
    return f"#{red:02x}{green:02x}{blue:02x}"


def apply(style, params):
    codes = [int(c) if c else 0 for c in params.split(";")]
    while codes:
        code = codes.pop(0)
        if code == 0:
            style.clear()
        elif code in (1, 2, 7):
            style[code] = True
        elif code in RESETS:
            for key in RESETS[code]:
                style.pop(key, None)
        elif 30 <= code <= 37 or 90 <= code <= 97:
            style["fg"] = ANSI[code - 30 if code < 90 else code - 82]
        elif 40 <= code <= 47 or 100 <= code <= 107:
            style["bg"] = ANSI[code - 40 if code < 100 else code - 92]
        elif code in (38, 48):
            style["fg" if code == 38 else "bg"] = extended(codes)
        elif code in (39, 49):
            style.pop("fg" if code == 39 else "bg", None)


def cells(lines, cols):
    grid = []
    for line in lines:
        style, row = {}, []
        for index, part in enumerate(SGR.split(line)):
            if index % 2:
                apply(style, part)
            else:
                row.extend((char, dict(style)) for char in part)
        grid.append((row + [(" ", {})] * cols)[:cols])
    return grid


def colours(style):
    fg, bg = style.get("fg", FG), style.get("bg", BG)
    return (bg, fg) if style.get(7) else (fg, bg)


def shape(char, x, y, fg):
    code = ord(char)
    if char in LINES:
        return line(char, x, y, fg)
    if char in SHADES:
        return rect(x, y + GAP, W, H - 2 * GAP, fg, SHADES[char])
    if 0x2581 <= code <= 0x2587:
        height = (H - 2 * GAP) * (code - 0x2580) / 8
        return rect(x, y + H - GAP - height, W, height, fg)
    if 0x2589 <= code <= 0x258F:
        return rect(x, y + GAP, W * (0x2590 - code) / 8, H - 2 * GAP, fg)
    if 0x2800 <= code <= 0x28FF:
        return braille(code - 0x2800, x, y, fg)
    return None


def rect(x, y, width, height, fill, opacity=1.0):
    alpha = "" if opacity == 1 else f' fill-opacity="{opacity}"'
    return f'<rect x="{x:.2f}" y="{y:.2f}" width="{width:.2f}" height="{height:.2f}" fill="{fill}"{alpha}/>'


def line(char, x, y, fg):
    cx, cy = x + W / 2, y + H / 2
    ends = {"l": (x, cy), "r": (x + W, cy), "u": (cx, y), "d": (cx, y + H)}
    sides = LINES[char]
    if char in ROUND:
        (ax, ay), (bx, by) = ends[sides[0]], ends[sides[1]]
        path = f"M{ax:.2f},{ay:.2f} Q{cx:.2f},{cy:.2f} {bx:.2f},{by:.2f}"
    else:
        path = " ".join(f"M{cx:.2f},{cy:.2f} L{ends[s][0]:.2f},{ends[s][1]:.2f}" for s in sides)
    return f'<path d="{path}" stroke="{fg}" stroke-width="1.2" fill="none"/>'


def braille(bits, x, y, fg):
    dots = []
    for bit, (dx, dy) in enumerate(BRAILLE):
        if bits >> bit & 1:
            cx, cy = x + W * (0.27 + 0.46 * dx), y + H * (0.125 + 0.25 * dy)
            dots.append(f'<circle cx="{cx:.2f}" cy="{cy:.2f}" r="1.5" fill="{fg}"/>')
    return "".join(dots)


def text_run(run, row):
    chars, style, start = run
    xs = " ".join(f"{PAD + (start + i) * W:.2f}" for i in range(len(chars)))
    weight = ' font-weight="bold"' if style.get(1) else ""
    dim = ' fill-opacity="0.6"' if style.get(2) else ""
    y = PAD + row * H + H * 0.74
    return f'<text x="{xs}" y="{y:.2f}" fill="{colours(style)[0]}"{weight}{dim}>{escape("".join(chars))}</text>'


def render_row(row, cells_row):
    out, run = [], None
    y = PAD + row * H
    for col, (char, style) in enumerate(cells_row):
        fg, bg = colours(style)
        x = PAD + col * W
        if bg != BG:
            out.append(rect(x, y, W + 0.3, H + 0.3, bg))
        drawn = None if char == " " else shape(char, x, y, fg)
        if run and (char == " " or drawn or run[1] != style):
            out.append(text_run(run, row))
            run = None
        if drawn:
            out.append(drawn)
        elif char != " ":
            run = run or ([], style, col)
            run[0].append(char)
    if run:
        out.append(text_run(run, row))
    return out


def main():
    cols, rows = int(sys.argv[1]), int(sys.argv[2])
    grid = cells(sys.stdin.read().split("\n")[:rows], cols)
    width, height = cols * W + 2 * PAD, rows * H + 2 * PAD
    font = "ui-monospace,SFMono-Regular,Menlo,Consolas,'DejaVu Sans Mono','Liberation Mono',monospace"
    print(f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0f}" height="{height:.0f}" '
          f'viewBox="0 0 {width:.0f} {height:.0f}" font-family="{font}" font-size="{FONT}">')
    print(f'<rect width="100%" height="100%" rx="8" fill="{BG}"/>')
    for row, cells_row in enumerate(grid):
        print("".join(render_row(row, cells_row)))
    print("</svg>")


if __name__ == "__main__":
    main()
