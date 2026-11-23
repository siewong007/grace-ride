#!/usr/bin/env python3
"""Generate Bloub-style seamless animated transparent GIF icons for Grace Ride.

Produces high-resolution supersampled animated GIFs (via rsvg-convert -> 2-pass ffmpeg
palettegen/paletteuse) into both:
  - frontend/public/icons/{avatars,badges,events}/
  - android/app/src/main/assets/icons/{avatars,badges,events}/

Clock-free deterministic sample(u) engine (u in [0, 1)) inspired by Bloub:
  - Volume-preserving squash & stretch
  - Pseudo-3D gazing pill eyes & natural double-blinks
  - Smooth organic silhouette morphing & orbital accent arcs
  - Secondary motion & crisp ink strokes for light and dark UI surfaces
"""

from __future__ import annotations

import argparse
import concurrent.futures
import math
import os
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
WEB_ICONS_DIR = ROOT / "frontend" / "public" / "icons"
ANDROID_ICONS_DIR = ROOT / "android" / "app" / "src" / "main" / "assets" / "icons"

# Art palette (solid opaque hex colors so 1-bit GIF alpha stays razor sharp)
C_BLUE = "#2563EB"
C_BLUE_DARK = "#1D4ED8"
C_SKY = "#60A5FA"
C_SKY_SOFT = "#DBEAFE"
C_RED = "#E8462F"
C_CORAL = "#FB7185"
C_CORAL_DEEP = "#F43F5E"
C_PINK = "#F472B6"
C_PINK_SOFT = "#FCE7F3"
C_BLUSH = "#FDA4AF"
C_AMBER = "#F59E0B"
C_GOLD = "#FBBF24"
C_CREAM = "#FEF3C7"
C_MINT = "#34D399"
C_GREEN = "#22C55E"
C_TEAL = "#14B8A6"
C_LAVENDER = "#C4B5FD"
C_PURPLE = "#8B5CF6"
C_INDIGO = "#4F46E5"
C_BROWN = "#92400E"
C_CARAMEL = "#D97706"
C_INK = "#1E293B"
C_WHITE = "#FFFFFF"
C_CYAN = "#22D3EE"
C_SLATE = "#475569"
C_SLATE_LIGHT = "#94A3B8"
C_MIST = "#E2E8F0"

AVATAR_COLORS = ["purple", "teal", "coral", "pink", "blue", "green", "amber", "gray", "white"]

GROK_SKINS = {
    "default": ("#18181B", "#3F3F46", "#FFFFFF"),
    "gray": ("#18181B", "#3F3F46", "#FFFFFF"),
    "purple": ("#7C3AED", "#5B21B6", "#FFFFFF"),
    "teal": ("#0D9488", "#115E59", "#FFFFFF"),
    "coral": ("#EA580C", "#9A3412", "#FFFFFF"),
    "pink": ("#DB2777", "#9D174D", "#FFFFFF"),
    "blue": ("#2563EB", "#1E40AF", "#FFFFFF"),
    "green": ("#16A34A", "#166534", "#FFFFFF"),
    "amber": ("#D97706", "#92400E", "#FFFFFF"),
    "white": ("#FFFFFF", "#CBD5E1", "#18181B"),
}

MUSE_PLUSH = {
    "default": ("#FEF3C7", "#FDE68A"),
    "amber": ("#FEF3C7", "#FDE68A"),
    "purple": ("#DDD6FE", "#C4B5FD"),
    "teal": ("#CCFBF1", "#99F6E4"),
    "coral": ("#FED7AA", "#FDBA74"),
    "pink": ("#FCE7F3", "#FBCFE8"),
    "blue": ("#DBEAFE", "#BFDBFE"),
    "green": ("#DCFCE7", "#BBF7D0"),
    "gray": ("#E4E4E7", "#D4D4D8"),
    "white": ("#FFFFFF", "#E2E8F0"),
}


# ---------------------------------------------------------------------------
# Clock-free periodic Bloub motion math (u in [0, 1))
# ---------------------------------------------------------------------------

def sin_w(u: float, freq: float = 1.0, phase: float = 0.0) -> float:
    return math.sin(2.0 * math.pi * (u * freq + phase))


def cos_w(u: float, freq: float = 1.0, phase: float = 0.0) -> float:
    return math.cos(2.0 * math.pi * (u * freq + phase))


def pulse01(u: float, freq: float = 1.0, phase: float = 0.0) -> float:
    return 0.5 - 0.5 * math.cos(2.0 * math.pi * (u * freq + phase))


def smoothstep(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def keyframed(u: float, pts: list[tuple[float, float]]) -> float:
    u = u % 1.0
    for i in range(len(pts) - 1):
        t0, v0 = pts[i]
        t1, v1 = pts[i + 1]
        if t0 <= u <= t1:
            if t1 <= t0:
                return v1
            return v0 + (v1 - v0) * smoothstep((u - t0) / (t1 - t0))
    return pts[-1][1]


def double_blink(u: float) -> float:
    return keyframed(
        u,
        [
            (0.00, 1.0),
            (0.58, 1.0),
            (0.62, 0.12),
            (0.66, 1.0),
            (0.71, 1.0),
            (0.75, 0.12),
            (0.80, 1.0),
            (1.00, 1.0),
        ],
    )


def gaze_cycle(u: float) -> tuple[float, float, float]:
    """Pseudo-3D gaze: (look_x_norm, look_y_norm, tilt_deg)."""
    lx = keyframed(
        u,
        [
            (0.00, 0.0),
            (0.14, 0.0),
            (0.26, 1.0),
            (0.42, 1.0),
            (0.56, -1.0),
            (0.72, -1.0),
            (0.86, 0.0),
            (1.00, 0.0),
        ],
    )
    ly = keyframed(
        u,
        [
            (0.00, 0.0),
            (0.20, -0.35),
            (0.40, 0.2),
            (0.62, -0.25),
            (0.82, 0.0),
            (1.00, 0.0),
        ],
    )
    tilt = lx * 4.5
    return lx, ly, tilt


def squash_stretch(sy: float) -> tuple[float, float]:
    sy = max(0.72, min(1.35, sy))
    return (1.0 / math.sqrt(sy), sy)


def xf_squash(cx: float, cy: float, sx: float, sy: float, dy: float = 0.0, rot: float = 0.0) -> str:
    return (
        f"translate({cx:.2f} {cy + dy:.2f}) rotate({rot:.2f}) "
        f"scale({sx:.4f} {sy:.4f}) translate({-cx:.2f} {-cy:.2f})"
    )


def svg_wrap(vb: int, body: str) -> str:
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {vb} {vb}" '
        f'width="{vb}" height="{vb}" fill="none">{body}</svg>'
    )


def heart_path(cx: float, cy: float, s: float) -> str:
    return (
        f"M {cx:.2f} {cy + 2.8 * s:.2f} "
        f"C {cx - 5.2 * s:.2f} {cy - 0.6 * s:.2f}, {cx - 5.5 * s:.2f} {cy - 4.8 * s:.2f}, {cx - 2.3 * s:.2f} {cy - 4.8 * s:.2f} "
        f"C {cx - 0.9 * s:.2f} {cy - 4.8 * s:.2f}, {cx:.2f} {cy - 3.6 * s:.2f}, {cx:.2f} {cy - 3.0 * s:.2f} "
        f"C {cx:.2f} {cy - 3.6 * s:.2f}, {cx + 0.9 * s:.2f} {cy - 4.8 * s:.2f}, {cx + 2.3 * s:.2f} {cy - 4.8 * s:.2f} "
        f"C {cx + 5.5 * s:.2f} {cy - 4.8 * s:.2f}, {cx + 5.2 * s:.2f} {cy - 0.6 * s:.2f}, {cx:.2f} {cy + 2.8 * s:.2f} Z"
    )


def star4(cx: float, cy: float, r: float, fill: str, rot: float = 0.0, scale: float = 1.0) -> str:
    if scale <= 0.05:
        return ""
    rr = r * scale
    ir = rr * 0.34
    pts = []
    for i in range(8):
        a = math.radians(rot + i * 45.0)
        rad = rr if i % 2 == 0 else ir
        pts.append(f"{cx + rad * math.cos(a):.2f},{cy + rad * math.sin(a):.2f}")
    return f'<polygon points="{" ".join(pts)}" fill="{fill}"/>'


def star5_path(cx: float, cy: float, r_out: float, r_in: float) -> str:
    pts = []
    for i in range(10):
        a = math.radians(-90.0 + i * 36.0)
        r = r_out if i % 2 == 0 else r_in
        pts.append(f"{cx + r * math.cos(a):.2f},{cy + r * math.sin(a):.2f}")
    return " ".join(pts)


def twinkle_pair(u: float, p1: tuple[float, float, float, str], p2: tuple[float, float, float, str]) -> str:
    s1 = 0.45 + 0.65 * pulse01(u, 1.0, 0.0)
    s2 = 0.45 + 0.65 * pulse01(u, 1.0, 0.5)
    return star4(p1[0], p1[1], p1[2], p1[3], u * 45.0, s1) + star4(p2[0], p2[1], p2[2], p2[3], -u * 45.0, s2)


def kawaii_eyes(
    cx1: float,
    cx2: float,
    cy: float,
    r: float,
    blink: float,
    lx: float = 0.0,
    ly: float = 0.0,
    fill: str = C_INK,
    catchlight: bool = True,
) -> str:
    rx = r
    ry = max(0.55, r * blink)
    dx = lx * r * 0.6
    dy = ly * r * 0.45
    out = (
        f'<ellipse cx="{cx1 + dx:.2f}" cy="{cy + dy:.2f}" rx="{rx:.2f}" ry="{ry:.2f}" fill="{fill}"/>'
        f'<ellipse cx="{cx2 + dx:.2f}" cy="{cy + dy:.2f}" rx="{rx:.2f}" ry="{ry:.2f}" fill="{fill}"/>'
    )
    if catchlight and blink > 0.65 and r >= 1.4:
        cr = r * 0.34
        out += (
            f'<circle cx="{cx1 + dx + r * 0.28:.2f}" cy="{cy + dy - r * 0.30:.2f}" r="{cr:.2f}" fill="{C_WHITE}"/>'
            f'<circle cx="{cx2 + dx + r * 0.28:.2f}" cy="{cy + dy - r * 0.30:.2f}" r="{cr:.2f}" fill="{C_WHITE}"/>'
        )
    return out


# ---------------------------------------------------------------------------
# AVATAR RENDERERS (viewBox 0 0 48 48)
# ---------------------------------------------------------------------------

def render_grok_avatar(u: float, color_key: str) -> str:
    skin, ring, eye = GROK_SKINS[color_key]
    bob = -2.0 * sin_w(u, 1.0)
    sy = 1.0 + 0.065 * sin_w(u, 1.0, 0.25)
    sx, sy = squash_stretch(sy)
    lx, ly, tilt = gaze_cycle(u)
    blink = double_blink(u)

    # Organic Bloub silhouette morphing between circle/squircle/pebble via superellipse rx
    rx = 11.5 + 3.8 * pulse01(u, 1.0, 0.15)
    w = 30.0
    h = 30.0
    body_rot = tilt * 0.7

    # Orbital Bloub reel arcs (sweeping mint + amber curves)
    orb1_rot = u * 360.0
    orb2_rot = -u * 360.0 + 140.0
    arcs = (
        f'<g transform="rotate({orb1_rot:.2f} 24 24)">'
        f'<path d="M 7.5 14 A 20 20 0 0 1 31 5.5" stroke="{C_MINT}" stroke-width="2.1" stroke-linecap="round"/>'
        f'<circle cx="33" cy="6.4" r="1.7" fill="{C_CYAN}"/>'
        f'</g>'
        f'<g transform="rotate({orb2_rot:.2f} 24 24)">'
        f'<path d="M 40.5 34 A 20 20 0 0 1 17 42.5" stroke="{C_AMBER}" stroke-width="2.1" stroke-linecap="round"/>'
        f'<circle cx="15" cy="41.4" r="1.6" fill="{C_GOLD}"/>'
        f'</g>'
    )

    # Pseudo-3D eye rig (shift + edge perspective narrowing)
    eye_shift_x = lx * 4.2
    eye_shift_y = ly * 2.2
    eye_w = 4.5 * (1.0 - 0.16 * abs(lx))
    eye_h = max(1.4, 9.8 * blink)
    eye_cy = 23.0 + eye_shift_y - eye_h * 0.5
    e1_x = 16.2 + eye_shift_x - eye_w * 0.5 + 2.25
    e2_x = 27.2 + eye_shift_x - eye_w * 0.5 + 2.25

    body_xf = xf_squash(24.0, 24.0, sx, sy, bob, body_rot)
    body = (
        f'<g transform="{body_xf}">'
        f'<rect x="{24 - w / 2:.2f}" y="{24 - h / 2:.2f}" width="{w:.2f}" height="{h:.2f}" '
        f'rx="{rx:.2f}" fill="{skin}" stroke="{ring}" stroke-width="1.6"/>'
        f'<ellipse cx="16.5" cy="14.5" rx="3.3" ry="1.4" transform="rotate(-24 16.5 14.5)" fill="{C_WHITE if color_key != "white" else C_SLATE_LIGHT}"/>'
        f'<rect x="{e1_x:.2f}" y="{eye_cy:.2f}" width="{eye_w:.2f}" height="{eye_h:.2f}" rx="{eye_w * 0.5:.2f}" fill="{eye}" transform="rotate({lx * -6:.1f} {e1_x + eye_w * 0.5:.2f} {23 + eye_shift_y:.2f})"/>'
        f'<rect x="{e2_x:.2f}" y="{eye_cy:.2f}" width="{eye_w:.2f}" height="{eye_h:.2f}" rx="{eye_w * 0.5:.2f}" fill="{eye}" transform="rotate({lx * -6:.1f} {e2_x + eye_w * 0.5:.2f} {23 + eye_shift_y:.2f})"/>'
        f'</g>'
    )
    return svg_wrap(48, arcs + body)


def render_muse_avatar(u: float, color_key: str) -> str:
    plush, shade = MUSE_PLUSH[color_key]
    hop = -2.4 * abs(sin_w(u, 1.0))
    sy = 1.0 + 0.08 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, tilt = gaze_cycle(u)
    arm_wave = 20.0 * sin_w(u, 1.0)
    foot_l = -1.6 * max(0.0, sin_w(u, 1.0))
    foot_r = -1.6 * max(0.0, -sin_w(u, 1.0))

    body_xf = xf_squash(24.0, 26.0, sx, sy, hop, tilt * 0.6)
    eyes = kawaii_eyes(18.6, 29.4, 21.4, 2.15, blink, lx * 0.65, ly * 0.5, C_INK, True)

    svg = (
        f'<g transform="{body_xf}">'
        f'<ellipse cx="18.2" cy="{39.6 + foot_l:.2f}" rx="4.4" ry="2.8" fill="{shade}" stroke="{C_INK}" stroke-width="1.6"/>'
        f'<ellipse cx="29.8" cy="{39.6 + foot_r:.2f}" rx="4.4" ry="2.8" fill="{shade}" stroke="{C_INK}" stroke-width="1.6"/>'
        f'<ellipse cx="9.4" cy="27" rx="3.4" ry="4.8" transform="rotate({24 + arm_wave:.2f} 11.4 25)" fill="{plush}" stroke="{C_INK}" stroke-width="1.6"/>'
        f'<ellipse cx="38.6" cy="27" rx="3.4" ry="4.8" transform="rotate({-24 - arm_wave:.2f} 36.6 25)" fill="{plush}" stroke="{C_INK}" stroke-width="1.6"/>'
        f'<path d="M24 7.8c2.2 0 3.8 1 4.5 2.1 6.8 1.4 10.7 6.8 10.7 14.6 0 9.4-5.4 15.2-15.2 15.2S8.8 33.9 8.8 24.5c0-7.8 3.9-13.2 10.7-14.6.7-1.1 2.3-2.1 4.5-2.1z" '
        f'fill="{plush}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round"/>'
        f'<ellipse cx="24" cy="30.6" rx="9" ry="6.2" fill="{shade}"/>'
        f'<ellipse cx="18.2" cy="14.2" rx="3.6" ry="1.5" transform="rotate(-18 18.2 14.2)" fill="{C_WHITE}"/>'
        f'{eyes}'
        f'<ellipse cx="14.5" cy="24.8" rx="3.0" ry="1.7" fill="{C_BLUSH}"/>'
        f'<ellipse cx="33.5" cy="24.8" rx="3.0" ry="1.7" fill="{C_BLUSH}"/>'
        f'<path d="M21 26.8c1.5 2 4.5 2 6 0" stroke="{C_INK}" stroke-width="1.6" stroke-linecap="round"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


def render_grace_car(u: float) -> str:
    bob = -1.8 * pulse01(u, 2.0)
    sy = 1.0 + 0.06 * sin_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    heart_dy = -3.8 * u
    heart_s = 0.65 * (1.0 - 0.55 * (u - 0.5) ** 2)
    wheel_rot = u * 360.0

    car_xf = xf_squash(24.0, 29.0, sx, sy, bob, sin_w(u, 1.0) * 1.6)
    eyes = kawaii_eyes(20.5, 27.5, 18.6, 1.4, blink, lx * 0.6, ly * 0.4)
    wheels = ""
    for wx in (15.0, 33.0):
        wheels += (
            f'<g transform="rotate({wheel_rot:.2f} {wx} 35)">'
            f'<circle cx="{wx}" cy="35" r="4.3" fill="{C_INK}"/>'
            f'<circle cx="{wx}" cy="35" r="2.0" fill="{C_CREAM}"/>'
            f'<line x1="{wx - 1.8}" y1="35" x2="{wx + 1.8}" y2="35" stroke="{C_INK}" stroke-width="0.9"/>'
            f'</g>'
        )
    heart = f'<path d="{heart_path(38.0, 10.5 + heart_dy, heart_s)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="0.9"/>'
    sparkles = twinkle_pair(u, (9.0, 11.5, 3.2, C_GOLD), (41.0, 17.5, 2.5, C_CYAN))
    body = (
        f'<g transform="{car_xf}">'
        f'<path d="M12 22c1.2-6 5.5-9.5 12-9.5s10.8 3.5 12 9.5z" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        f'<path d="M15 21.5c1-4 4.2-6.5 9-6.5s8 2.5 9 6.5z" fill="{C_WHITE}"/>'
        f'{eyes}'
        f'<path d="M22.3 20c1.1 1 2.3 1 3.4 0" stroke="{C_INK}" stroke-width="1.3" stroke-linecap="round"/>'
        f'<rect x="7" y="21" width="34" height="13" rx="6.5" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<rect x="9" y="29" width="30" height="3.2" rx="1.6" fill="{C_CREAM}"/>'
        f'<circle cx="11.2" cy="25.5" r="2.6" fill="{C_GOLD}"/>'
        f'<circle cx="36.8" cy="25.5" r="2.6" fill="{C_GOLD}"/>'
        f'<ellipse cx="15" cy="27.2" rx="2" ry="1.1" fill="{C_PINK}"/>'
        f'<ellipse cx="33" cy="27.2" rx="2" ry="1.1" fill="{C_PINK}"/>'
        f'<rect x="20.5" y="23.2" width="7" height="5" rx="1.5" fill="{C_WHITE}"/>'
        f'<path d="M24 24.1v3.2M22.4 25.7h3.2" stroke="{C_RED}" stroke-width="1.5" stroke-linecap="round"/>'
        f'</g>'
    )
    return svg_wrap(48, sparkles + heart + body + wheels)


def render_peace_dove(u: float) -> str:
    bob = -2.2 * sin_w(u, 1.0)
    flap = 16.0 * sin_w(u, 2.0)
    sy = 1.0 + 0.05 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    body_xf = xf_squash(24.0, 25.0, sx, sy, bob, -2.5 * cos_w(u, 1.0))

    eye_ry = max(0.5, 1.75 * blink)
    svg = (
        twinkle_pair(u, (38.5, 10.5, 3.6, C_GOLD), (10.5, 12.5, 2.5, C_CYAN))
        + f'<g transform="{body_xf}">'
        f'<path d="M15 22c-5.5-5.5-8.5-2.5-6.5 3.5 1.5 4.2 5.5 6 9.5 5.5z" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round" transform="rotate({flap:.2f} 17 24)"/>'
        f'<path d="M12 29l-6 4 4.5 2.5 4.5-3.5z" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round"/>'
        f'<ellipse cx="24.5" cy="25" rx="12.5" ry="11" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<path d="M18 22c2.5-6.5 9-7.5 10.5-1.5 1 4-2.5 8-8 7.5-2-.2-3-2.5-2.5-6z" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.7" transform="rotate({-flap * 0.8:.2f} 22 23)"/>'
        f'<ellipse cx="{29.5 + lx * 0.7:.2f}" cy="{21 + ly * 0.5:.2f}" rx="1.75" ry="{eye_ry:.2f}" fill="{C_INK}"/>'
        + (f'<circle cx="{30.1 + lx * 0.7:.2f}" cy="{20.4 + ly * 0.5:.2f}" r="0.6" fill="{C_WHITE}"/>' if blink > 0.6 else "")
        + f'<ellipse cx="27.5" cy="24.2" rx="2.2" ry="1.3" fill="{C_BLUSH}"/>'
        f'<path d="M36.5 21.5l4.2 1.5-4 1.8z" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.4" stroke-linejoin="round"/>'
        f'<g transform="rotate({6 * sin_w(u, 1.0, 0.2):.2f} 38.5 23.5)">'
        f'<path d="M38.5 23.5c2 2.2 3.5 4.5 3.5 6.5" stroke="{C_GREEN}" stroke-width="1.6" stroke-linecap="round"/>'
        f'<ellipse cx="41.2" cy="25.5" rx="2.1" ry="1.2" transform="rotate(-25 41.2 25.5)" fill="{C_MINT}" stroke="{C_INK}" stroke-width="0.9"/>'
        f'<ellipse cx="38.5" cy="27.2" rx="2.1" ry="1.2" transform="rotate(20 38.5 27.2)" fill="{C_GREEN}" stroke="{C_INK}" stroke-width="0.9"/>'
        f'</g></g>'
    )
    return svg_wrap(48, svg)


def render_sunny_cloud(u: float) -> str:
    spin = u * 360.0
    bob = -1.8 * sin_w(u, 1.0)
    sy = 1.0 + 0.06 * cos_w(u, 1.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)

    rays = "".join(
        f'<line x1="31" y1="6.8" x2="31" y2="10.2" stroke="{C_AMBER}" stroke-width="2.4" stroke-linecap="round" transform="rotate({spin + i * 45:.2f} 31 18.5)"/>'
        for i in range(8)
    )
    sun_r = 7.3 + 0.5 * pulse01(u, 2.0)
    cloud_xf = xf_squash(24.0, 29.0, sx, sy, bob, 0.0)
    eyes = kawaii_eyes(19.0, 29.0, 28.0, 1.65, blink, lx * 0.7, ly * 0.5)
    svg = (
        rays
        + f'<circle cx="31" cy="18.5" r="{sun_r:.2f}" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
        + f'<g transform="{cloud_xf}">'
        f'<path d="M14 36h20a7 7 0 0 0 1-13.9 8.5 8.5 0 0 0-16.4-1.8A6.8 6.8 0 0 0 14 36z" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        + eyes
        + f'<ellipse cx="16.2" cy="30.8" rx="2.1" ry="1.2" fill="{C_BLUSH}"/>'
        f'<ellipse cx="31.8" cy="30.8" rx="2.1" ry="1.2" fill="{C_BLUSH}"/>'
        f'<path d="M21.8 30.2c1.2 1.8 3.2 1.8 4.4 0" stroke="{C_INK}" stroke-width="1.6" stroke-linecap="round"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


def render_happy_bear(u: float) -> str:
    bob = -1.6 * pulse01(u, 2.0)
    tilt = 4.2 * sin_w(u, 1.0)
    ear_wig = 6.0 * sin_w(u, 1.0, 0.2)
    sy = 1.0 + 0.06 * sin_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    heart_dy = -2.8 * pulse01(u, 1.0)
    head_xf = xf_squash(24.0, 25.0, sx, sy, bob, tilt)
    eyes = kawaii_eyes(18.2, 29.8, 22.5, 1.8, blink, lx * 0.65, ly * 0.4)

    svg = (
        twinkle_pair(u, (9.5, 9.5, 2.8, C_GOLD), (39.5, 19.0, 2.3, C_SKY))
        + f'<path d="{heart_path(38.5, 9.5 + heart_dy, 0.65)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="0.9"/>'
        + f'<g transform="{head_xf}">'
        f'<g transform="rotate({ear_wig:.2f} 16 18)">'
        f'<circle cx="13.5" cy="14" r="5.5" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<circle cx="13.5" cy="14" r="2.7" fill="{C_CREAM}"/></g>'
        f'<g transform="rotate({-ear_wig:.2f} 32 18)">'
        f'<circle cx="34.5" cy="14" r="5.5" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<circle cx="34.5" cy="14" r="2.7" fill="{C_CREAM}"/></g>'
        f'<ellipse cx="24" cy="24.5" rx="13.5" ry="11.5" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<ellipse cx="24" cy="27.2" rx="6.2" ry="4.5" fill="{C_CREAM}"/>'
        f'<ellipse cx="24" cy="25.5" rx="2.1" ry="1.4" fill="{C_BROWN}"/>'
        f'<path d="M21.8 28.4c1 1.1 3.4 1.1 4.4 0" stroke="{C_BROWN}" stroke-width="1.5" stroke-linecap="round"/>'
        + eyes
        + f'<ellipse cx="14.5" cy="25.8" rx="2.2" ry="1.2" fill="{C_CORAL}"/>'
        f'<ellipse cx="33.5" cy="25.8" rx="2.2" ry="1.2" fill="{C_CORAL}"/>'
        f'<path d="M14.5 34.5c3.5 2.5 15.5 2.5 19 0l1.5 4.2c-5 2.2-17 2.2-22 0z" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round"/>'
        f'<rect x="28.5" y="36" width="4.5" height="6.5" rx="1.8" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.4" transform="rotate({5 * sin_w(u, 1.0, 0.35):.2f} 30.7 36)"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


def render_fellowship_cat(u: float) -> str:
    bob = -1.6 * pulse01(u, 2.0)
    tilt = 3.6 * sin_w(u, 1.0)
    sy = 1.0 + 0.06 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    whisker_r = 4.5 * sin_w(u, 2.0)
    head_xf = xf_squash(24.0, 27.0, sx, sy, bob, tilt)
    eyes = kawaii_eyes(18.2, 29.8, 25.5, 2.1, blink, lx * 0.7, ly * 0.5)

    svg = (
        twinkle_pair(u, (39.0, 10.5, 3.6, C_GOLD), (9.0, 12.0, 2.4, C_PINK))
        + f'<g transform="{head_xf}">'
        f'<path d="M12 20L10.5 9.5l8.5 5.5z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        f'<path d="M13 17.5L12.2 12.2l4.5 3z" fill="{C_PINK}"/>'
        f'<path d="M36 20l1.5-10.5L29 15z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        f'<path d="M35 17.5l.8-5.3-4.5 3z" fill="{C_PINK}"/>'
        f'<ellipse cx="24" cy="27" rx="14" ry="12" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<path d="M24 15.5v3.2M21.2 16.2l.8 2.6M26.8 16.2l-.8 2.6" stroke="{C_CARAMEL}" stroke-width="1.8" stroke-linecap="round"/>'
        + eyes
        + f'<polygon points="24,28.8 22.3,27 25.7,27" fill="{C_CORAL}"/>'
        f'<path d="M21 30.2c1 1.3 2.5 1 3-.6.5 1.6 2 1.9 3 .6" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
        f'<ellipse cx="14.2" cy="28.5" rx="2.3" ry="1.3" fill="{C_PINK}"/>'
        f'<ellipse cx="33.8" cy="28.5" rx="2.3" ry="1.3" fill="{C_PINK}"/>'
        f'<g stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round">'
        f'<g transform="rotate({whisker_r:.2f} 12 27.5)"><line x1="12" y1="26.5" x2="6.5" y2="25.2"/><line x1="12" y1="29.5" x2="6.5" y2="30.8"/></g>'
        f'<g transform="rotate({-whisker_r:.2f} 36 27.5)"><line x1="36" y1="26.5" x2="41.5" y2="25.2"/><line x1="36" y1="29.5" x2="41.5" y2="30.8"/></g>'
        f'</g></g>'
    )
    return svg_wrap(48, svg)


def render_joy_puppy(u: float) -> str:
    bob = -2.0 * abs(sin_w(u, 1.0))
    sy = 1.0 + 0.065 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    tail_wag = 18.0 * sin_w(u, 3.0)
    ear_flop = 8.0 * sin_w(u, 2.0, 0.2)
    head_xf = xf_squash(24.0, 25.0, sx, sy, bob, 2.5 * sin_w(u, 1.0))
    eyes = kawaii_eyes(18.2, 29.8, 21.5, 1.8, blink, lx * 0.65, ly * 0.4)

    svg = (
        twinkle_pair(u, (8.5, 10.5, 2.7, C_GOLD), (39.5, 12.0, 2.5, C_TEAL))
        + f'<path d="M35.5 35c4.5-2 6-5.5 4.2-8.5" stroke="{C_CARAMEL}" stroke-width="3.5" stroke-linecap="round" transform="rotate({tail_wag:.2f} 35.5 35)"/>'
        f'<g transform="{head_xf}">'
        f'<ellipse cx="10.5" cy="22" rx="4.2" ry="8.2" transform="rotate({14 + ear_flop:.2f} 10.5 16)" fill="{C_BROWN}" stroke="{C_INK}" stroke-width="1.7"/>'
        f'<ellipse cx="37.5" cy="22" rx="4.2" ry="8.2" transform="rotate({-14 - ear_flop:.2f} 37.5 16)" fill="{C_BROWN}" stroke="{C_INK}" stroke-width="1.7"/>'
        f'<ellipse cx="24" cy="23.5" rx="12.5" ry="11.5" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
        f'<circle cx="18.2" cy="21.5" r="4.2" fill="{C_CREAM}"/>'
        f'<ellipse cx="24" cy="27.2" rx="5.8" ry="4.2" fill="{C_CREAM}"/>'
        + eyes
        + f'<ellipse cx="24" cy="25.2" rx="2.2" ry="1.5" fill="{C_INK}"/>'
        f'<path d="M22.5 28.8c0 {1.7 + 0.5 * pulse01(u, 2.0):.2f} 3 {1.7 + 0.5 * pulse01(u, 2.0):.2f} 3 0z" fill="{C_CORAL}"/>'
        f'<path d="M21.5 27.6c1 1.2 4 1.2 5 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
        f'<path d="M15 34.5c4.5 2 13.5 2 18 0" stroke="{C_TEAL}" stroke-width="3.2" stroke-linecap="round"/>'
        f'<circle cx="{24 + 1.0 * sin_w(u, 2.0):.2f}" cy="37.5" r="2.4" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.3"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


def render_star_lantern(u: float) -> str:
    bob = -1.8 * sin_w(u, 1.0)
    rot = 5.0 * sin_w(u, 1.0, 0.25)
    sy = 1.0 + 0.06 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    star_xf = xf_squash(24.0, 24.0, sx, sy, bob, rot)
    eyes = kawaii_eyes(20.2, 27.8, 22.8, 1.65, blink, lx * 0.6, ly * 0.4)

    a1 = u * 2 * math.pi
    a2 = a1 + 2.1
    a3 = a1 + 4.2
    orbs = (
        star4(24 + 18.5 * math.cos(a1), 24 + 18.5 * math.sin(a1), 3.1, C_CYAN, u * 90)
        + star4(24 + 18.5 * math.cos(a2), 24 + 18.5 * math.sin(a2), 2.8, C_PINK, -u * 90)
        + star4(24 + 18.5 * math.cos(a3), 24 + 18.5 * math.sin(a3), 2.8, C_MINT, u * 90)
    )
    svg = (
        orbs
        + f'<g transform="{star_xf}">'
        f'<polygon points="{star5_path(24, 23.5, 17.2, 8.2)}" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        f'<circle cx="24" cy="24" r="{6.4 + 0.8 * pulse01(u, 2.0):.2f}" fill="{C_CREAM}"/>'
        + eyes
        + f'<ellipse cx="17.8" cy="25.5" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
        f'<ellipse cx="30.2" cy="25.5" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
        f'<path d="M22 25.8c1 1.4 3 1.4 4 0" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


def render_blossom_sprout(u: float) -> str:
    sway = 6.5 * sin_w(u, 1.0)
    leaf_w = 6.0 * sin_w(u, 1.0, 0.25)
    sy = 1.0 + 0.055 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)
    dew_y = 11.0 - 2.0 * sin_w(u, 1.0, 0.3)

    head_xf = xf_squash(24.0, 31.0, sx, sy, 0.0, sway)
    eyes = kawaii_eyes(19.5, 28.5, 22.0, 1.65, blink, lx * 0.6, ly * 0.4)
    svg = (
        twinkle_pair(u, (9.5, 12.0, 2.6, C_GOLD), (39.5, 20.0, 2.4, C_PINK))
        + f'<circle cx="38" cy="{dew_y:.2f}" r="2.2" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.1"/>'
        f'<path d="M24 31v10.5" stroke="{C_GREEN}" stroke-width="3.2" stroke-linecap="round"/>'
        f'<path d="M24 37c-5.5-1-9.5-4.5-9.5-7.5 3.5-1 7.5 1.5 9.5 7.5z" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.5" stroke-linejoin="round" transform="rotate({leaf_w:.2f} 24 37)"/>'
        f'<path d="M24 37c5.5-1 9.5-4.5 9.5-7.5-3.5-1-7.5 1.5-9.5 7.5z" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.5" stroke-linejoin="round" transform="rotate({-leaf_w:.2f} 24 37)"/>'
        f'<g transform="{head_xf}">'
        f'<path d="M12 19c0-5 4-9 6-9 2.5 0 4.5 3 6 5.5 1.5-2.5 3.5-5.5 6-5.5 2 0 6 4 6 9 0 7.5-5.5 12.5-12 12.5S12 26.5 12 19z" fill="{C_PINK}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
        f'<path d="M18.5 15c2.2 2.5 8.8 2.5 11 0" fill="{C_PINK_SOFT}"/>'
        + eyes
        + f'<ellipse cx="16.8" cy="24.8" rx="2" ry="1.1" fill="{C_CORAL}"/>'
        f'<ellipse cx="31.2" cy="24.8" rx="2" ry="1.1" fill="{C_CORAL}"/>'
        f'<path d="M21.8 25.2c1.2 1.4 3.2 1.4 4.4 0" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
        f'</g>'
    )
    return svg_wrap(48, svg)


# ---------------------------------------------------------------------------
# SHARED CLASSIC ANIMATED ICON GENERATORS (for 48x48 avatars/events & 56x56 badges)
# ---------------------------------------------------------------------------

def render_classic_icon(key: str, u: float, vb: int = 48) -> str:
    """Bloub-inspired colorful animated 2D flat-vector icon on crisp transparent bg."""
    sc = vb / 48.0
    cx = vb / 2.0
    bob = -2.0 * sin_w(u, 1.0)
    sy = 1.0 + 0.065 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, tilt = gaze_cycle(u)
    inner = ""

    if key == "car":
        xf = xf_squash(24, 28, sx, sy, -1.6 * pulse01(u, 2.0), 1.5 * sin_w(u, 1.0))
        wrot = u * 360.0
        inner = (
            twinkle_pair(u, (9, 13, 2.8, C_GOLD), (39, 13, 2.4, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M12 24l3.5-8h17l3.5 8z" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="6.5" y="23" width="35" height="12" rx="5.5" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="9" y="29.5" width="30" height="2.6" rx="1.3" fill="{C_CREAM}"/>'
            f'<circle cx="11" cy="26.8" r="2.2" fill="{C_GOLD}"/>'
            f'<circle cx="37" cy="26.8" r="2.2" fill="{C_GOLD}"/>'
            f'</g>'
            + "".join(
                f'<g transform="rotate({wrot:.2f} {wx} 35.5)"><circle cx="{wx}" cy="35.5" r="4.3" fill="{C_INK}"/><circle cx="{wx}" cy="35.5" r="2" fill="{C_CREAM}"/><line x1="{wx-1.8}" y1="35.5" x2="{wx+1.8}" y2="35.5" stroke="{C_INK}" stroke-width="0.9"/></g>'
                for wx in (14.5, 33.5)
            )
        )
    elif key == "heart":
        xf = xf_squash(24, 26, sx, sy, bob, 2.5 * sin_w(u, 1.0))
        h2_y = 11.5 - 3.5 * pulse01(u, 1.0)
        inner = (
            twinkle_pair(u, (10, 14, 2.8, C_GOLD), (39, 15, 2.4, C_PINK))
            + f'<path d="{heart_path(34.5, h2_y, 0.72)}" fill="{C_PINK}" stroke="{C_INK}" stroke-width="1.0"/>'
            + f'<g transform="{xf}">'
            f'<path d="{heart_path(24, 29, 3.1)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            + kawaii_eyes(19.8, 28.2, 23.5, 1.5, blink, lx * 0.5, ly * 0.3)
            + f'<path d="M22 26.2c1 1.2 3 1.2 4 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "star":
        xf = xf_squash(24, 24, sx, sy, bob, 6.0 * sin_w(u, 1.0))
        inner = (
            twinkle_pair(u, (9, 11, 3.0, C_CYAN), (39, 37, 2.8, C_PINK))
            + f'<g transform="{xf}">'
            f'<polygon points="{star5_path(24, 24, 17.5, 8.2)}" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<circle cx="24" cy="24" r="5.5" fill="{C_CREAM}"/>'
            + kawaii_eyes(21.2, 26.8, 23.2, 1.35, blink, lx * 0.5, ly * 0.3)
            + f'<path d="M22.4 25.8c.9 1 2.3 1 3.2 0" stroke="{C_INK}" stroke-width="1.3" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "music":
        b1 = -2.8 * sin_w(u, 1.0)
        b2 = -2.8 * sin_w(u, 1.0, 0.4)
        rot = 4.5 * sin_w(u, 1.0)
        inner = (
            twinkle_pair(u, (9, 13, 2.8, C_GOLD), (39, 34, 2.5, C_CYAN))
            + f'<g transform="rotate({rot:.2f} 24 24)">'
            f'<path d="M18 33V13l18-3.5v20" stroke="{C_INK}" stroke-width="5.2" stroke-linecap="round" stroke-linejoin="round"/>'
            f'<path d="M18 33V13l18-3.5v20" stroke="{C_PURPLE}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>'
            f'<path d="M18 14l18-3.5" stroke="{C_PINK}" stroke-width="3.6" stroke-linecap="round"/>'
            f'<ellipse cx="14.2" cy="{34 + b1 * 0.4:.2f}" rx="5.2" ry="4" transform="rotate(-18 14.2 34)" fill="{C_PURPLE}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<ellipse cx="32.2" cy="{30.5 + b2 * 0.4:.2f}" rx="5.2" ry="4" transform="rotate(-18 32.2 30.5)" fill="{C_PINK}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'</g>'
        )
    elif key == "sun":
        spin = u * 360.0
        orb_r = 10.2 + 0.6 * pulse01(u, 2.0)
        rays = "".join(
            f'<line x1="24" y1="5.2" x2="24" y2="9.8" stroke="{C_AMBER}" stroke-width="2.6" stroke-linecap="round" transform="rotate({spin + i * 45:.2f} 24 24)"/>'
            for i in range(8)
        )
        inner = (
            rays
            + f'<circle cx="24" cy="24" r="{orb_r:.2f}" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="24" r="6.8" fill="{C_CREAM}"/>'
            + kawaii_eyes(20.6, 27.4, 23.0, 1.45, blink, lx * 0.6, ly * 0.4)
            + f'<path d="M22.2 26c1 1.2 2.6 1.2 3.6 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
        )
    elif key == "leaf":
        sway = 7.5 * sin_w(u, 1.0)
        dew = 15 + 3.0 * pulse01(u, 1.0)
        xf = xf_squash(24, 34, sx, sy, 0.0, sway)
        inner = (
            twinkle_pair(u, (10, 14, 2.6, C_MINT), (38, 12, 2.4, C_GOLD))
            + f'<g transform="{xf}">'
            f'<path d="M12 35C12 19 22 10 37 10c0 16-9 26-25 25z" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M10 38c6-7 13-13 21-19" stroke="{C_GREEN}" stroke-width="2.2" stroke-linecap="round"/>'
            f'<path d="M20 28l6-1M24 23l1-5" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
            f'</g>'
            f'<circle cx="32" cy="{dew:.2f}" r="2.3" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.1"/>'
        )
    elif key == "coffee":
        xf = xf_squash(22, 30, sx, sy, -1.2 * pulse01(u, 2.0), 0.0)
        st1 = -3.0 * pulse01(u, 1.0, 0.0)
        st2 = -3.0 * pulse01(u, 1.0, 0.5)
        inner = (
            f'<path d="M16 14c-1.5-2.5 1.5-4.5 0-7" stroke="{C_SLATE_LIGHT}" stroke-width="2.1" stroke-linecap="round" transform="translate(0 {st1:.2f})"/>'
            f'<path d="M23 13c-1.5-2.5 1.5-4.5 0-7" stroke="{C_AMBER}" stroke-width="2.1" stroke-linecap="round" transform="translate(0 {st2:.2f})"/>'
            f'<path d="M30 14c-1.5-2.5 1.5-4.5 0-7" stroke="{C_SLATE_LIGHT}" stroke-width="2.1" stroke-linecap="round" transform="translate(0 {st1:.2f})"/>'
            f'<g transform="{xf}">'
            f'<path d="M34 21h4.2a4.3 4.3 0 0 1 0 8.6H34" stroke="{C_INK}" stroke-width="2.2" fill="none"/>'
            f'<path d="M11 19h23v12.5a8.5 8.5 0 0 1-8.5 8.5h-6A8.5 8.5 0 0 1 11 31.5V19z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="11" y="19" width="23" height="5" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<path d="{heart_path(22.5, 31.5, 0.85)}" fill="{C_CORAL}"/>'
            f'</g>'
        )
    elif key == "book" or key == "bookOpen":
        xf = xf_squash(24, 26, sx, sy, bob, 2.0 * sin_w(u, 1.0))
        flap = -2.2 * pulse01(u, 2.0)
        inner = (
            twinkle_pair(u, (24, 9, 3.2, C_GOLD), (39, 13, 2.2, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M7 15c6 0 13 2 17 5 4-3 11-5 17-5v20c-6 0-13 2-17 4.5C20 37 13 35 7 35z" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M24 20v19.5" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M10.5 21.5c3.5.3 7 1.2 9.5 2.5M10.5 27c3.5.3 7 1.2 9.5 2.5" stroke="{C_SKY}" stroke-width="1.8" stroke-linecap="round"/>'
            f'<path d="M37.5 {21.5 + flap * 0.4:.2f}c-3.5.3-7 1.2-9.5 2.5M37.5 {27 + flap * 0.4:.2f}c-3.5.3-7 1.2-9.5 2.5" stroke="{C_SKY}" stroke-width="1.8" stroke-linecap="round"/>'
            f'<path d="M31 16v9l2.2-1.8 2.2 1.8v-9z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'</g>'
        )
    elif key == "church":
        xf = xf_squash(24, 30, sx, sy, -1.3 * pulse01(u, 2.0), 0.0)
        bell_rot = 12.0 * sin_w(u, 2.0)
        cross_s = 1.0 + 0.1 * pulse01(u, 2.0)
        inner = (
            twinkle_pair(u, (10, 12, 2.8, C_GOLD), (38, 12, 2.8, C_GOLD))
            + f'<g transform="{xf}">'
            f'<g transform="translate(24 10) scale({cross_s:.3f}) translate(-24 -10)">'
            f'<path d="M24 5.5v8M20.5 8.8h7" stroke="{C_AMBER}" stroke-width="2.6" stroke-linecap="round"/></g>'
            f'<path d="M16 24l8-8 8 8v17H16z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M8 29l8-4v16H8zM32 25l8 4v12h-8z" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round"/>'
            f'<path d="M20.5 41v-7a3.5 3.5 0 0 1 7 0v7z" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<circle cx="24" cy="23.5" r="2.3" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.2" transform="rotate({bell_rot:.2f} 24 21)"/>'
            f'</g>'
        )
    elif key == "bike":
        bob_b = -1.6 * pulse01(u, 2.0)
        wrot = u * 360.0
        wheels = ""
        for wx in (13.0, 35.0):
            wheels += (
                f'<g transform="rotate({wrot:.2f} {wx} 33)">'
                f'<circle cx="{wx}" cy="33" r="7.5" fill="{C_SKY_SOFT}" stroke="{C_TEAL}" stroke-width="2.3"/>'
                f'<circle cx="{wx}" cy="33" r="7.5" stroke="{C_INK}" stroke-width="1.3"/>'
                f'<line x1="{wx-5}" y1="33" x2="{wx+5}" y2="33" stroke="{C_INK}" stroke-width="1.1"/>'
                f'<line x1="{wx}" y1="28" x2="{wx}" y2="38" stroke="{C_INK}" stroke-width="1.1"/>'
                f'<circle cx="{wx}" cy="33" r="1.8" fill="{C_INK}"/>'
                f'</g>'
            )
        inner = (
            twinkle_pair(u, (10, 12, 2.4, C_GOLD), (39, 13, 2.4, C_CYAN))
            + wheels
            + f'<g transform="translate(0 {bob_b:.2f})">'
            f'<path d="M13 33l8-11h9l5 11M21 22l-2-4h-3M30 22l1.5-5h4" stroke="{C_INK}" stroke-width="4.2" stroke-linecap="round" stroke-linejoin="round"/>'
            f'<path d="M13 33l8-11h9l5 11M21 22l-2-4h-3M30 22l1.5-5h4" stroke="{C_TEAL}" stroke-width="2.0" stroke-linecap="round" stroke-linejoin="round"/>'
            f'<circle cx="24" cy="31" r="2.4" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'</g>'
        )
    elif key == "smile":
        xf = xf_squash(24, 24, sx, sy, bob, tilt * 0.7)
        eyes = kawaii_eyes(18.5, 29.5, 21.5, 2.0, blink, lx * 0.9, ly * 0.5)
        inner = (
            twinkle_pair(u, (8.5, 11, 2.6, C_GOLD), (39.5, 11, 2.6, C_PINK))
            + f'<g transform="{xf}">'
            f'<circle cx="24" cy="24" r="16" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="24" r="13.5" fill="{C_CREAM}"/>'
            + eyes
            + f'<ellipse cx="14.5" cy="25.5" rx="2.3" ry="1.3" fill="{C_CORAL}"/>'
            f'<ellipse cx="33.5" cy="25.5" rx="2.3" ry="1.3" fill="{C_CORAL}"/>'
            f'<path d="M18.5 27.5c2 3.2 9 3.2 11 0" stroke="{C_INK}" stroke-width="1.8" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "flower":
        spin = 12.0 * sin_w(u, 1.0)
        xf = xf_squash(24, 24, sx, sy, bob, spin)
        petals = "".join(
            f'<ellipse cx="24" cy="12" rx="4.5" ry="7.2" fill="{C_PINK if i % 2 == 0 else C_PINK_SOFT}" stroke="{C_INK}" stroke-width="1.5" transform="rotate({i * 60} 24 24)"/>'
            for i in range(6)
        )
        inner = (
            f'<g transform="{xf}">'
            + petals
            + f'<circle cx="24" cy="24" r="6.8" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.6"/>'
            + kawaii_eyes(21.6, 26.4, 23.2, 1.2, blink, lx * 0.5, ly * 0.3)
            + f'<path d="M22.4 25.5c.9.9 2.3.9 3.2 0" stroke="{C_INK}" stroke-width="1.2" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "trophy":
        xf = xf_squash(24, 25, sx, sy, bob, 4.0 * sin_w(u, 1.0))
        inner = (
            twinkle_pair(u, (9, 12, 3.2, C_GOLD), (39, 13, 2.8, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M14 14h-4a3 3 0 0 0 0 6h4M34 14h4a3 3 0 0 1 0 6h-4" stroke="{C_INK}" stroke-width="2" fill="{C_AMBER}"/>'
            f'<path d="M14 11h20v11c0 6-4.5 10-10 10s-10-4-10-10z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="21.5" y="32" width="5" height="5.5" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<rect x="16" y="37" width="16" height="4.2" rx="1.8" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<polygon points="{star5_path(24, 20.5, 4.2, 2.0)}" fill="{C_CREAM}"/>'
            f'</g>'
        )
    elif key == "medal":
        swing = 7.0 * sin_w(u, 1.0)
        xf = xf_squash(24, 28, sx, sy, bob, swing)
        inner = (
            twinkle_pair(u, (10, 22, 2.8, C_GOLD), (38, 22, 2.8, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M16 7l5 12h6L22 7z" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.5" stroke-linejoin="round"/>'
            f'<path d="M32 7l-5 12h-6l5-12z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.5" stroke-linejoin="round"/>'
            f'<circle cx="24" cy="29" r="11.5" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="29" r="8.4" fill="{C_CREAM}" stroke="{C_AMBER}" stroke-width="1.3"/>'
            f'<polygon points="{star5_path(24, 29, 5.2, 2.4)}" fill="{C_AMBER}"/>'
            f'</g>'
        )
    elif key == "shield":
        pulse = 1.0 + 0.06 * pulse01(u, 2.0)
        xf = xf_squash(24, 24, pulse, pulse, -1.2 * sin_w(u, 1.0), 0.0)
        inner = (
            twinkle_pair(u, (10, 12, 2.8, C_GOLD), (38, 12, 2.6, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M24 8l14 4.5v10c0 8.5-5.8 14.5-14 17.5-8.2-3-14-9-14-17.5v-10z" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M24 12l10 3.3v7c0 6.2-4.2 10.8-10 13-5.8-2.2-10-6.8-10-13v-7z" fill="{C_SKY_SOFT}"/>'
            f'<path d="M19.5 24l3.2 3.2 6.2-6.2" stroke="{C_BLUE}" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>'
            f'</g>'
        )
    elif key == "crown":
        xf = xf_squash(24, 26, sx, sy, bob, 4.0 * sin_w(u, 1.0))
        inner = (
            twinkle_pair(u, (10, 11, 2.8, C_CYAN), (38, 11, 2.8, C_PINK))
            + f'<g transform="{xf}">'
            f'<path d="M9 34L12 16l7 7 5-10 5 10 7-7 3 18z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="9" y="33" width="30" height="5" rx="2" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<circle cx="24" cy="26" r="2.4" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1"/>'
            f'<circle cx="15.5" cy="27.5" r="1.9" fill="{C_CYAN}" stroke="{C_INK}" stroke-width="1"/>'
            f'<circle cx="32.5" cy="27.5" r="1.9" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1"/>'
            f'</g>'
        )
    elif key == "sparkles":
        s1 = 0.65 + 0.50 * pulse01(u, 1.0, 0.0)
        s2 = 0.55 + 0.55 * pulse01(u, 1.0, 0.33)
        s3 = 0.55 + 0.55 * pulse01(u, 1.0, 0.66)
        inner = (
            star4(21, 23, 12.5, C_GOLD, 12 * sin_w(u, 1.0), s1)
            + f'<circle cx="21" cy="23" r="2.2" fill="{C_CREAM}"/>'
            + star4(35, 13, 6.8, C_CYAN, -20 * sin_w(u, 1.0), s2)
            + star4(34, 35, 6.2, C_PINK, 20 * sin_w(u, 1.0), s3)
        )
    elif key == "flame":
        flick = 5.0 * sin_w(u, 2.0)
        xf = xf_squash(24, 36, sx, sy, 0.0, flick * 0.6)
        inner = (
            twinkle_pair(u, (11, 13, 2.6, C_GOLD), (37, 15, 2.4, C_CORAL))
            + f'<g transform="{xf}">'
            f'<path d="M24 7c3 6.5 11 9.5 11 19a11 11 0 0 1-22 0c0-5 2.8-9.5 6-12.5 0 3.5 1.8 5 3.5 5-.5-4.5.5-8.5 1.5-11.5z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M24 18c2.2 3.5 6 6 6 11a6 6 0 0 1-12 0c0-3.2 1.8-6.2 4-8.2.2 2 1 3 2 3z" fill="{C_GOLD}" transform="rotate({-flick:.2f} 24 31)"/>'
            f'<ellipse cx="24" cy="31.5" rx="3" ry="3.5" fill="{C_CREAM}"/>'
            f'</g>'
        )
    elif key == "zap":
        pulse = 1.0 + 0.09 * pulse01(u, 2.0)
        xf = xf_squash(24, 24, pulse, pulse, -1.2 * sin_w(u, 2.0), 3.0 * sin_w(u, 2.0))
        inner = (
            twinkle_pair(u, (11, 15, 2.8, C_CYAN), (37, 33, 2.8, C_AMBER))
            + f'<g transform="{xf}">'
            f'<polygon points="27,6 12,26 23,26 20,42 36,21 25,21" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'</g>'
        )
    elif key == "clock":
        jiggle = 4.0 * sin_w(u, 2.0)
        hand = u * 360.0
        xf = xf_squash(24, 24, sx, sy, -1.0 * pulse01(u, 2.0), jiggle)
        inner = (
            f'<g transform="{xf}">'
            f'<circle cx="24" cy="24" r="15.5" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="24" r="11.8" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<line x1="24" y1="24" x2="24" y2="15" stroke="{C_TEAL}" stroke-width="2.2" stroke-linecap="round" transform="rotate({hand:.2f} 24 24)"/>'
            f'<line x1="24" y1="24" x2="29.5" y2="24" stroke="{C_INK}" stroke-width="2.0" stroke-linecap="round" transform="rotate({hand / 12.0:.2f} 24 24)"/>'
            f'<circle cx="24" cy="24" r="1.9" fill="{C_CORAL}"/>'
            f'</g>'
        )
    elif key == "target":
        pulse = 1.0 + 0.06 * pulse01(u, 2.0)
        dart_d = -3.0 * pulse01(u, 1.0)
        inner = (
            f'<g transform="{xf_squash(24, 24, pulse, pulse)}">'
            f'<circle cx="24" cy="24" r="15.5" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="24" r="10.8" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="24" cy="24" r="5.8" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'</g>'
            f'<g transform="translate({-dart_d:.2f} {dart_d:.2f})">'
            f'<line x1="24" y1="24" x2="38" y2="10" stroke="{C_INK}" stroke-width="4.0" stroke-linecap="round"/>'
            f'<line x1="24" y1="24" x2="38" y2="10" stroke="{C_MIST}" stroke-width="1.8" stroke-linecap="round"/>'
            f'<polygon points="34,10 38,10 38,14" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.1"/>'
            f'<circle cx="24" cy="24" r="2.2" fill="{C_RED}"/>'
            f'</g>'
        )
    elif key == "gift":
        wig = 5.0 * sin_w(u, 2.0)
        lid_dy = -3.2 * pulse01(u, 1.0)
        xf = xf_squash(24, 30, sx, sy, bob, wig * 0.4)
        inner = (
            twinkle_pair(u, (10, 11, 2.8, C_GOLD), (38, 12, 2.6, C_CYAN))
            + f'<g transform="{xf}">'
            f'<rect x="11" y="21" width="26" height="17" rx="3" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="21.5" y="21" width="5" height="17" fill="{C_GOLD}"/>'
            f'<g transform="translate(0 {lid_dy:.2f}) rotate({wig * 0.5:.2f} 24 18)">'
            f'<rect x="9" y="15.5" width="30" height="6" rx="2" fill="{C_PINK}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="21.5" y="15.5" width="5" height="6" fill="{C_GOLD}"/>'
            f'<path d="M24 15.5c-4-5-8-2-5 0h10c3-2-1-5-5 0z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'</g></g>'
        )
    elif key == "thumbsUp":
        xf = xf_squash(24, 26, sx, sy, bob, -7.0 * sin_w(u, 1.0))
        inner = (
            twinkle_pair(u, (11, 12, 3.0, C_GOLD), (38, 15, 2.6, C_CYAN))
            + f'<g transform="{xf}">'
            f'<rect x="9.5" y="21" width="7.5" height="16" rx="2.5" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M17 22l4.5-10.5c1-2.2 4.5-1.5 4.5 1.5v7h9.5c2.2 0 3.8 2 3.3 4.2l-2 9.2A4 4 0 0 1 33 37H17z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'</g>'
        )
    elif key == "compass":
        needle = 24.0 * sin_w(u, 1.0)
        xf = xf_squash(24, 25, sx, sy, -1.0 * pulse01(u, 2.0), 0.0)
        inner = (
            twinkle_pair(u, (9, 12, 2.6, C_CYAN), (39, 36, 2.6, C_GOLD))
            + f'<g transform="{xf}">'
            f'<circle cx="24" cy="9" r="2.6" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<circle cx="24" cy="25.5" r="14.5" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="24" cy="25.5" r="11" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.3"/>'
            f'<g transform="rotate({needle:.2f} 24 25.5)">'
            f'<polygon points="24,15.5 27,25.5 21,25.5" fill="{C_RED}" stroke="{C_INK}" stroke-width="1.1"/>'
            f'<polygon points="24,35.5 27,25.5 21,25.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.1"/>'
            f'</g>'
            f'<circle cx="24" cy="25.5" r="2.4" fill="{C_INK}"/>'
            f'</g>'
        )
    elif key == "moon":
        rock = 7.5 * sin_w(u, 1.0)
        xf = xf_squash(24, 24, sx, sy, bob, rock)
        inner = (
            twinkle_pair(u, (34, 14, 3.3, C_GOLD), (38, 28, 2.5, C_CYAN))
            + f'<g transform="{xf}">'
            f'<path d="M29 10a14 14 0 1 0 11 22.5A11.5 11.5 0 0 1 29 10z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'</g>'
        )
    elif key == "heartHandshake":
        pulse = 1.0 + 0.07 * pulse01(u, 2.0)
        xf = xf_squash(24, 25, pulse, pulse, bob, 0.0)
        inner = (
            twinkle_pair(u, (10, 12, 2.8, C_GOLD), (38, 12, 2.6, C_PINK))
            + f'<g transform="{xf}">'
            f'<path d="{heart_path(24, 29, 3.0)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M16 23l5 4.5 3-2.5 3 2.5 5-4.5" stroke="{C_CREAM}" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>'
            f'</g>'
        )
    elif key == "utensils":
        l_dy = -2.4 * pulse01(u, 1.0, 0.0)
        r_dy = -2.4 * pulse01(u, 1.0, 0.5)
        inner = (
            twinkle_pair(u, (9, 13, 2.6, C_GOLD), (39, 13, 2.6, C_CORAL))
            + f'<g transform="translate(0 {l_dy:.2f}) rotate({-3 * sin_w(u, 1.0):.2f} 17 26)">'
            f'<path d="M13.5 9.5v9a3.5 3.5 0 0 0 7 0v-9M17 9.5v17.5" stroke="{C_INK}" stroke-width="4.2" stroke-linecap="round"/>'
            f'<path d="M13.5 9.5v9a3.5 3.5 0 0 0 7 0v-9M17 9.5v17.5" stroke="{C_MIST}" stroke-width="2.0" stroke-linecap="round"/>'
            f'<rect x="14.5" y="26.5" width="5" height="12" rx="2.4" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'</g>'
            f'<g transform="translate(0 {r_dy:.2f}) rotate({3 * sin_w(u, 1.0):.2f} 31 26)">'
            f'<line x1="31" y1="20" x2="31" y2="27" stroke="{C_INK}" stroke-width="4.2" stroke-linecap="round"/>'
            f'<line x1="31" y1="20" x2="31" y2="27" stroke="{C_MIST}" stroke-width="2.0" stroke-linecap="round"/>'
            f'<ellipse cx="31" cy="14.5" rx="4.4" ry="6.2" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<rect x="28.5" y="26.5" width="5" height="12" rx="2.4" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'</g>'
        )
    elif key == "cake":
        flick = 0.85 + 0.3 * pulse01(u, 3.0)
        xf = xf_squash(24, 30, sx, sy, -1.2 * pulse01(u, 2.0), 0.0)
        inner = (
            twinkle_pair(u, (10, 12, 2.8, C_CYAN), (38, 13, 2.6, C_PINK))
            + f'<g transform="{xf}">'
            f'<rect x="22" y="13" width="4" height="7" rx="1" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<ellipse cx="24" cy="{10.5:.2f}" rx="{2.2 * flick:.2f}" ry="{3.3 * flick:.2f}" fill="{C_GOLD}" stroke="{C_CORAL}" stroke-width="1.1"/>'
            f'<rect x="11" y="20" width="26" height="8" rx="3" fill="{C_PINK_SOFT}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<rect x="9" y="28" width="30" height="10" rx="3" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M12 24q3 2.5 6 0t6 0 6 0 6 0" stroke="{C_PINK}" stroke-width="2.2" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "users":
        xf1 = xf_squash(17, 28, sx, sy, bob, 2.0 * sin_w(u, 1.0))
        xf2 = xf_squash(31, 28, sx, sy, -bob * 0.7, -2.0 * sin_w(u, 1.0))
        inner = (
            twinkle_pair(u, (24, 9, 2.8, C_GOLD), (40, 14, 2.2, C_MINT))
            + f'<g transform="{xf2}">'
            f'<circle cx="31.5" cy="18" r="5.5" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<path d="M23.5 36c0-4.8 3.5-8 8-8s8 3.2 8 8z" fill="{C_TEAL}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round"/>'
            f'</g>'
            f'<g transform="{xf1}">'
            f'<circle cx="17.5" cy="17.5" r="6.2" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M8.5 37c0-5.4 4-9 9-9s9 3.6 9 9z" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            + kawaii_eyes(15.5, 19.5, 17.2, 1.15, blink, lx * 0.5, ly * 0.3)
            + f'</g>'
        )
    elif key == "home":
        xf = xf_squash(24, 28, sx, sy, -1.4 * pulse01(u, 2.0), 0.0)
        heart_dy = -3.5 * pulse01(u, 1.0)
        inner = (
            f'<path d="{heart_path(34, 11 + heart_dy, 0.7)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="0.9"/>'
            + f'<g transform="{xf}">'
            f'<path d="M11 23v16h26V23z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M7.5 24.5L24 10l16.5 14.5z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="20.5" y="28" width="7" height="11" rx="1.5" fill="{C_CARAMEL}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<circle cx="24" cy="21" r="2.6" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'</g>'
        )
    elif key == "footprints":
        s_l = 1.0 + 0.14 * max(0.0, sin_w(u, 1.0))
        s_r = 1.0 + 0.14 * max(0.0, -sin_w(u, 1.0))
        dy_l = -2.6 * max(0.0, sin_w(u, 1.0))
        dy_r = -2.6 * max(0.0, -sin_w(u, 1.0))
        inner = (
            f'<g transform="{xf_squash(16.5, 22, s_l, s_l, dy_l, -8)}">'
            f'<ellipse cx="16.5" cy="18.5" rx="4.5" ry="6.8" fill="{C_TEAL}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<ellipse cx="16.5" cy="29.5" rx="3.4" ry="4" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'</g>'
            f'<g transform="{xf_squash(31.5, 26, s_r, s_r, dy_r, 8)}">'
            f'<ellipse cx="31.5" cy="22.5" rx="4.5" ry="6.8" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<ellipse cx="31.5" cy="33.5" rx="3.4" ry="4" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'</g>'
        )
    elif key == "waves":
        dx1 = 4.0 * sin_w(u, 1.0)
        dx2 = -4.0 * sin_w(u, 1.0)
        inner = (
            twinkle_pair(u, (12, 11, 2.5, C_GOLD), (37, 12, 2.5, C_CYAN))
            + f'<path d="M5 {20 + 1.5 * cos_w(u, 1.0):.2f}q5-4 10 0t10 0 10 0 10 0" stroke="{C_CYAN}" stroke-width="3.2" stroke-linecap="round" transform="translate({dx1 * 0.4:.2f} 0)"/>'
            f'<path d="M5 {28 + 1.5 * sin_w(u, 1.0):.2f}q5-4 10 0t10 0 10 0 10 0" stroke="{C_BLUE}" stroke-width="3.4" stroke-linecap="round" transform="translate({dx2 * 0.4:.2f} 0)"/>'
            f'<path d="M5 {36 - 1.5 * cos_w(u, 1.0):.2f}q5-4 10 0t10 0 10 0 10 0" stroke="{C_TEAL}" stroke-width="3.2" stroke-linecap="round" transform="translate({dx1 * 0.4:.2f} 0)"/>'
        )
    elif key == "tent":
        xf = xf_squash(24, 30, sx, sy, -1.2 * pulse01(u, 2.0), 0.0)
        flag_wig = 10.0 * sin_w(u, 2.0)
        inner = (
            twinkle_pair(u, (10, 12, 2.8, C_GOLD), (38, 14, 2.5, C_CYAN))
            + f'<g transform="{xf}">'
            f'<line x1="24" y1="8.5" x2="24" y2="16" stroke="{C_INK}" stroke-width="3.6" stroke-linecap="round"/>'
            f'<line x1="24" y1="8.5" x2="24" y2="16" stroke="{C_MIST}" stroke-width="1.4" stroke-linecap="round"/>'
            f'<polygon points="24,8.5 32,11 24,13.5" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.2" transform="rotate({flag_wig:.2f} 24 11)"/>'
            f'<polygon points="24,15 7,38 41,38" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<polygon points="24,23 17,38 31,38" fill="{C_INK}"/>'
            f'<polygon points="24,25 20,38 28,38" fill="{C_GOLD}"/>'
            f'</g>'
        )
    elif key == "mountain":
        sun_dy = -2.2 * sin_w(u, 1.0)
        cloud_dx = 3.2 * sin_w(u, 1.0)
        inner = (
            f'<circle cx="34" cy="{15 + sun_dy:.2f}" r="5.2" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<polygon points="7,38 20,16 33,38" fill="{C_TEAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<polygon points="21,38 32,20 42,38" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<polygon points="20,16 15.5,24 20,22 24.5,24" fill="{C_WHITE}"/>'
            f'<ellipse cx="{15 + cloud_dx:.2f}" cy="13.5" rx="6.5" ry="3.2" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
        )
    elif key == "dumbbell":
        lift = -3.0 * pulse01(u, 2.0)
        rot = 6.0 * sin_w(u, 1.0)
        inner = (
            twinkle_pair(u, (11, 12, 2.8, C_GOLD), (37, 12, 2.8, C_CYAN))
            + f'<g transform="translate(0 {lift:.2f}) rotate({rot:.2f} 24 25)">'
            f'<line x1="12" y1="25" x2="36" y2="25" stroke="{C_INK}" stroke-width="5.2" stroke-linecap="round"/>'
            f'<line x1="12" y1="25" x2="36" y2="25" stroke="{C_SLATE_LIGHT}" stroke-width="2.4" stroke-linecap="round"/>'
            f'<rect x="7.5" y="18" width="5.5" height="14" rx="2" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<rect x="12.5" y="15.5" width="5" height="19" rx="2" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<rect x="35" y="18" width="5.5" height="14" rx="2" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<rect x="30.5" y="15.5" width="5" height="19" rx="2" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'</g>'
        )
    elif key == "gamepad":
        xf = xf_squash(24, 25, sx, sy, bob, 4.0 * sin_w(u, 1.0))
        b_col = C_GOLD if pulse01(u, 2.0) > 0.5 else C_CORAL
        inner = (
            twinkle_pair(u, (10, 12, 2.5, C_CYAN), (38, 12, 2.5, C_PINK))
            + f'<g transform="{xf}">'
            f'<rect x="7" y="16" width="34" height="19" rx="9.5" fill="{C_PURPLE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M15.5 22v7M12 25.5h7" stroke="{C_WHITE}" stroke-width="2.5" stroke-linecap="round"/>'
            f'<circle cx="30.5" cy="23.5" r="2.1" fill="{b_col}"/>'
            f'<circle cx="34.5" cy="27" r="2.1" fill="{C_MINT}"/>'
            f'</g>'
        )
    elif key == "film":
        xf = xf_squash(24, 25, sx, sy, -1.2 * pulse01(u, 2.0), 0.0)
        spin = u * 360.0
        inner = (
            f'<g transform="{xf}">'
            f'<rect x="9" y="13" width="30" height="24" rx="3" fill="{C_SLATE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="16.5" y="16.5" width="15" height="17" rx="1.5" fill="{C_GOLD}"/>'
            f'<circle cx="24" cy="25" r="4" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.2" transform="rotate({spin:.2f} 24 25)"/>'
            + "".join(
                f'<rect x="{x}" y="{y}" width="3.5" height="3.2" rx="0.6" fill="{C_CREAM}"/>'
                for x in (11, 33.5)
                for y in (16.5, 23.4, 30.2)
            )
            + f'</g>'
        )
    elif key == "camera":
        xf = xf_squash(24, 26, sx, sy, -1.4 * pulse01(u, 2.0), 2.5 * sin_w(u, 1.0))
        flash = 0.3 + 0.9 * pulse01(u, 2.0)
        inner = (
            star4(36, 13, 4.4, C_GOLD, u * 90, flash)
            + f'<g transform="{xf}">'
            f'<path d="M9 17h6l2.5-4h13l2.5 4h6a3 3 0 0 1 3 3v15a3 3 0 0 1-3 3H9a3 3 0 0 1-3-3V20a3 3 0 0 1 3-3z" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<circle cx="24" cy="27" r="7.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<circle cx="24" cy="27" r="4.5" fill="{C_BLUE}"/>'
            f'<circle cx="25.6" cy="25.4" r="1.4" fill="{C_WHITE}"/>'
            f'</g>'
        )
    elif key == "bus":
        xf = xf_squash(24, 24, sx, sy, -1.5 * pulse01(u, 2.0), 1.0 * sin_w(u, 1.0))
        wrot = u * 360.0
        inner = (
            twinkle_pair(u, (9, 10, 2.5, C_GOLD), (39, 10, 2.5, C_CYAN))
            + f'<g transform="{xf}">'
            f'<rect x="7" y="13" width="34" height="21" rx="5" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="10" y="16.5" width="12" height="8" rx="1.5" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.3"/>'
            f'<rect x="25" y="16.5" width="13" height="8" rx="1.5" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.3"/>'
            f'<rect x="7" y="28" width="34" height="3.2" fill="{C_BLUE}"/>'
            f'</g>'
            + "".join(
                f'<g transform="rotate({wrot:.2f} {wx} 35.5)"><circle cx="{wx}" cy="35.5" r="4.2" fill="{C_INK}"/><circle cx="{wx}" cy="35.5" r="1.8" fill="{C_CREAM}"/></g>'
                for wx in (15, 33)
            )
        )
    elif key == "baby":
        xf = xf_squash(24, 25, sx, sy, bob, 4.0 * sin_w(u, 1.0))
        eyes = kawaii_eyes(19.5, 28.5, 25.0, 1.6, blink, lx * 0.5, ly * 0.3)
        inner = (
            twinkle_pair(u, (9, 13, 2.7, C_GOLD), (39, 13, 2.7, C_PINK))
            + f'<g transform="{xf}">'
            f'<circle cx="24" cy="25.5" r="13.5" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M24 12c-2.5 0-3.5 3.5-1 5.2 2 1.2 4.2-.2 3.6-2.4" stroke="{C_CARAMEL}" stroke-width="1.8" stroke-linecap="round"/>'
            + eyes
            + f'<ellipse cx="16.5" cy="27.8" rx="2.1" ry="1.2" fill="{C_BLUSH}"/>'
            f'<ellipse cx="31.5" cy="27.8" rx="2.1" ry="1.2" fill="{C_BLUSH}"/>'
            f'<path d="M21.5 29.2c1.2 1.5 3.8 1.5 5 0" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
            f'</g>'
        )
    elif key == "calendar":
        xf = xf_squash(24, 26, sx, sy, -1.2 * pulse01(u, 2.0), 0.0)
        pop = 1.0 + 0.16 * pulse01(u, 2.0)
        inner = (
            f'<g transform="{xf}">'
            f'<rect x="9" y="12" width="30" height="27" rx="4.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M9 16.5a4.5 4.5 0 0 1 4.5-4.5h21a4.5 4.5 0 0 1 4.5 4.5V21H9z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<line x1="17" y1="9" x2="17" y2="15" stroke="{C_INK}" stroke-width="4.0" stroke-linecap="round"/>'
            f'<line x1="17" y1="9" x2="17" y2="15" stroke="{C_MIST}" stroke-width="1.8" stroke-linecap="round"/>'
            f'<line x1="31" y1="9" x2="31" y2="15" stroke="{C_INK}" stroke-width="4.0" stroke-linecap="round"/>'
            f'<line x1="31" y1="9" x2="31" y2="15" stroke="{C_MIST}" stroke-width="1.8" stroke-linecap="round"/>'
            f'<g transform="translate(24 30) scale({pop:.3f}) translate(-24 -30)">'
            f'<circle cx="24" cy="30" r="5.5" fill="{C_MINT}"/>'
            f'<path d="M21.4 30l1.9 1.9 3.6-3.6" stroke="{C_WHITE}" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/>'
            f'</g></g>'
        )
    else:
        raise ValueError(f"Unknown classic icon key: {key}")

    if abs(sc - 1.0) > 1e-4:
        return svg_wrap(vb, f'<g transform="scale({sc:.4f})">{inner}</g>')
    return svg_wrap(vb, inner)


# ---------------------------------------------------------------------------
# FEATURED ROLE BADGE RENDERERS (viewBox 0 0 56 56)
# ---------------------------------------------------------------------------

def render_featured_badge(key: str, u: float) -> str:
    bob = -2.2 * sin_w(u, 1.0)
    sy = 1.0 + 0.06 * cos_w(u, 2.0)
    sx, sy = squash_stretch(sy)
    blink = double_blink(u)
    lx, ly, _ = gaze_cycle(u)

    if key == "badge_joy_rider":
        car_xf = xf_squash(28, 30, sx, sy, -1.6 * pulse01(u, 2.0), 1.4 * sin_w(u, 1.0))
        h_dy = -3.0 * pulse01(u, 1.0)
        body = (
            twinkle_pair(u, (10, 14, 3.2, C_GOLD), (46, 12, 3.0, C_CYAN))
            + f'<path d="{heart_path(30.5, 9.5 + h_dy, 0.72)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="0.9"/>'
            + f'<g transform="{car_xf}">'
            f'<ellipse cx="22.5" cy="15.5" rx="2.1" ry="5" transform="rotate(-10 22.5 15.5)" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<ellipse cx="22.5" cy="15.5" rx="1" ry="2.8" transform="rotate(-10 22.5 15.5)" fill="{C_PINK}"/>'
            f'<ellipse cx="28" cy="15" rx="2.1" ry="5" transform="rotate(8 28 15)" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<ellipse cx="28" cy="15" rx="1" ry="2.8" transform="rotate(8 28 15)" fill="{C_PINK}"/>'
            f'<circle cx="25.5" cy="21" r="5.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.6"/>'
            + kawaii_eyes(23.8, 27.2, 20.5, 1.15, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="22.4" cy="22.3" rx="1.4" ry="0.8" fill="{C_BLUSH}"/>'
            f'<ellipse cx="28.6" cy="22.3" rx="1.4" ry="0.8" fill="{C_BLUSH}"/>'
            f'<path d="M11 31c1.5-5.5 5-8 10.5-8h13c5.5 0 9 2.5 10.5 8z" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round"/>'
            f'<rect x="8" y="29" width="40" height="12.5" rx="6" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<rect x="11" y="36" width="34" height="3" rx="1.5" fill="{C_CREAM}"/>'
            f'<circle cx="12.5" cy="33" r="2.2" fill="{C_GOLD}"/>'
            f'<circle cx="43.5" cy="33" r="2.2" fill="{C_GOLD}"/>'
            f'<path d="M25.5 31.5l1.1 1.1c.6.6 1.5 0 1.9-.6.5.6 1.3 1.2 1.9.6l1.1-1.1" stroke="{C_CORAL}" stroke-width="1.5" stroke-linecap="round"/>'
            f'</g>'
            f'<circle cx="17" cy="42.5" r="4.2" fill="{C_INK}"/><circle cx="17" cy="42.5" r="1.8" fill="{C_CREAM}"/>'
            f'<circle cx="39" cy="42.5" r="4.2" fill="{C_INK}"/><circle cx="39" cy="42.5" r="1.8" fill="{C_CREAM}"/>'
        )
        return svg_wrap(56, body)

    if key == "badge_fellowship_sprout":
        bell_rot = 9.0 * sin_w(u, 1.0)
        fl_sway = 6.0 * sin_w(u, 1.0, 0.25)
        n_dy = -2.8 * pulse01(u, 1.0)
        body = (
            twinkle_pair(u, (43, 14, 3.0, C_GOLD), (45, 34, 2.2, C_PINK))
            + f'<g transform="translate(0 {n_dy:.2f})">'
            f'<path d="M12 15v-5.5l4-1.2v5.5" stroke="{C_TEAL}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>'
            f'<circle cx="10.8" cy="15.2" r="1.6" fill="{C_TEAL}"/></g>'
            f'<g transform="rotate({bell_rot:.2f} 28 8)">'
            f'<path d="M28 10c-6.5 0-10.5 5-10.5 12.5 0 4.5-2 7-3.5 8.5h28c-1.5-1.5-3.5-4-3.5-8.5C38.5 15 34.5 10 28 10z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<circle cx="28" cy="8.5" r="2.2" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<circle cx="28" cy="33" r="3" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.5"/>'
            + kawaii_eyes(24, 32, 22, 1.5, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="21.5" cy="24.5" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
            f'<ellipse cx="34.5" cy="24.5" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
            f'<path d="M26 24.8c1 1.3 3 1.3 4 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
            f'</g>'
            f'<g transform="rotate({fl_sway:.2f} 28 45)">'
            f'<ellipse cx="20" cy="43" rx="5.5" ry="2.6" transform="rotate(-18 20 43)" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<ellipse cx="36" cy="43" rx="5.5" ry="2.6" transform="rotate(18 36 43)" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="28" cy="40" r="3.4" fill="{C_PINK}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="24.5" cy="42.5" r="3.2" fill="{C_PINK_SOFT}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="31.5" cy="42.5" r="3.2" fill="{C_PINK_SOFT}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="28" cy="42.5" r="2.2" fill="{C_GOLD}"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "badge_punctual_clock":
        jiggle = 6.0 * sin_w(u, 3.0) * (1.0 if 0.15 < u < 0.55 else 0.25)
        pop = 1.0 + 0.14 * pulse01(u, 2.0)
        xf = xf_squash(28, 28.5, sx, sy, -1.2 * pulse01(u, 2.0), jiggle)
        hand_rot = u * 360.0
        body = (
            f'<g transform="{xf}">'
            f'<circle cx="15.5" cy="14.5" r="5" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<circle cx="40.5" cy="14.5" r="5" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<path d="M18 42l-3 5M38 42l3 5" stroke="{C_INK}" stroke-width="4.2" stroke-linecap="round"/>'
            f'<path d="M18 42l-3 5M38 42l3 5" stroke="{C_AMBER}" stroke-width="2.0" stroke-linecap="round"/>'
            f'<circle cx="28" cy="28.5" r="15.5" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="28" cy="28.5" r="11.8" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<line x1="28" y1="28.5" x2="28" y2="20.5" stroke="{C_TEAL}" stroke-width="2" stroke-linecap="round" transform="rotate({hand_rot:.2f} 28 28.5)"/>'
            + kawaii_eyes(23, 33, 29.5, 1.4, blink, lx * 0.4, ly * 0.3)
            + f'<ellipse cx="20.5" cy="32" rx="1.8" ry="1" fill="{C_BLUSH}"/>'
            f'<ellipse cx="35.5" cy="32" rx="1.8" ry="1" fill="{C_BLUSH}"/>'
            f'<path d="M26 32.8c1 1.2 3 1.2 4 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
            f'</g>'
            f'<g transform="translate(43 40.5) scale({pop:.3f}) translate(-43 -40.5)">'
            f'<circle cx="43" cy="40.5" r="7" fill="{C_GREEN}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<path d="M39.8 40.5l2.2 2.2 4.3-4.3" stroke="{C_WHITE}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "badge_warm_welcome":
        h_dy = -3.2 * pulse01(u, 1.0)
        xf = xf_squash(24, 32, sx, sy, bob, 2.0 * sin_w(u, 1.0))
        body = (
            twinkle_pair(u, (11, 16, 2.8, C_PINK), (46, 17, 2.8, C_GOLD))
            + f'<path d="{heart_path(28.5, 10 + h_dy, 0.7)}" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="0.9"/>'
            f'<path d="M36.5 18c-3.2 0-5.2 2.3-5.2 2.3S29.3 18 26.1 18c-3.4 0-6.1 2.7-6.1 6.3 0 6.5 11.3 14.2 11.3 14.2S42.6 30.8 42.6 24.3c0-3.6-2.7-6.3-6.1-6.3z" fill="{C_PINK_SOFT}" stroke="{C_INK}" stroke-width="1.7" stroke-linejoin="round" transform="rotate({5 * sin_w(u, 1.0):.2f} 32 28)"/>'
            f'<g transform="{xf}">'
            f'<path d="M29.5 21.5c-3.6 0-5.8 2.6-5.8 2.6s-2.2-2.6-5.8-2.6c-3.9 0-7 3.1-7 7.1 0 7.5 12.8 16.4 12.8 16.4s12.8-8.9 12.8-16.4c0-4-3.1-7.1-7-7.1z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            + kawaii_eyes(19.5, 28, 30.5, 1.6, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="16.8" cy="33" rx="2" ry="1.1" fill="{C_CREAM}"/>'
            f'<ellipse cx="30.7" cy="33" rx="2" ry="1.1" fill="{C_CREAM}"/>'
            f'<path d="M21.8 33.4c1.2 1.3 2.8 1.3 4 0" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "moon":
        rock = 5.5 * sin_w(u, 1.0)
        xf = xf_squash(28, 28, sx, sy, bob * 0.7, rock)
        body = (
            twinkle_pair(u, (12, 13, 3.2, C_GOLD), (44, 12, 2.8, C_SKY))
            + f'<g transform="{xf}">'
            f'<path d="M15 24c-3.5 9 1.5 20.5 11.5 23.5 8.5 2.5 17.5-1.5 21-9-6 2.5-13 1.5-18-3.5-5-5-6-12-3.5-18-5.5 1.2-9.5 3.5-11 7z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<ellipse cx="30" cy="27.5" rx="9" ry="9.5" fill="{C_PURPLE}" stroke="{C_INK}" stroke-width="1.7"/>'
            f'<path d="M23.5 19.5l-1.5-3.5 4.2 2zM36.5 19.5l1.5-3.5-4.2 2z" fill="{C_INDIGO}" stroke="{C_INK}" stroke-width="1.4" stroke-linejoin="round"/>'
            f'<ellipse cx="30" cy="31.5" rx="5.2" ry="4.5" fill="{C_CREAM}"/>'
            f'<circle cx="26.2" cy="24.5" r="3.6" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.3"/>'
            f'<circle cx="33.8" cy="24.5" r="3.6" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.3"/>'
            + kawaii_eyes(26.5, 33.5, 24.6, 1.6, blink, lx * 0.5, ly * 0.3)
            + f'<polygon points="30,27.8 28.5,25.8 31.5,25.8" fill="{C_AMBER}"/>'
            f'<ellipse cx="22.8" cy="27.5" rx="1.6" ry="0.9" fill="{C_PINK}"/>'
            f'<ellipse cx="37.2" cy="27.5" rx="1.6" ry="0.9" fill="{C_PINK}"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "award":
        rib = 6.0 * sin_w(u, 1.0, 0.2)
        xf = xf_squash(28, 23, sx, sy, bob, 3.0 * sin_w(u, 1.0))
        body = (
            twinkle_pair(u, (10, 18, 2.8, C_CYAN), (46, 18, 2.8, C_PINK))
            + f'<g transform="rotate({rib:.2f} 28 32)">'
            f'<path d="M21 32l-4.5 16 6.5-3.2 4 4.2 2-15z" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round"/>'
            f'<path d="M35 32l4.5 16-6.5-3.2-4 4.2-2-15z" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round"/></g>'
            f'<g transform="{xf}">'
            f'<circle cx="28" cy="23" r="14.5" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="28" cy="23" r="11" fill="{C_CREAM}" stroke="{C_AMBER}" stroke-width="1.5"/>'
            + kawaii_eyes(23.5, 32.5, 22, 1.6, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="21" cy="24.8" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
            f'<ellipse cx="35" cy="24.8" rx="1.9" ry="1.1" fill="{C_CORAL}"/>'
            f'<path d="M25.8 25.2c1.2 1.4 3.2 1.4 4.4 0" stroke="{C_INK}" stroke-width="1.5" stroke-linecap="round"/>'
            f'<path d="M22.5 12l2.5 3.5 3-4.5 3 4.5 2.5-3.5-.8 5h-9.4z" fill="{C_AMBER}" stroke="{C_INK}" stroke-width="1.4" stroke-linejoin="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "heart":
        # Caring Companion coral carpool van (BadgeCarpoolLove)
        xf = xf_squash(28, 30, sx, sy, -1.7 * pulse01(u, 2.0), 1.4 * sin_w(u, 1.0))
        h1_dy = -3.0 * pulse01(u, 1.0, 0.0)
        h2_dy = -3.0 * pulse01(u, 1.0, 0.5)
        body = (
            f'<path d="{heart_path(19.5, 11.5 + h1_dy, 0.65)}" fill="{C_PINK}" stroke="{C_INK}" stroke-width="0.9"/>'
            f'<path d="{heart_path(37.5, 9.5 + h2_dy, 0.65)}" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="0.9"/>'
            f'<g transform="{xf}">'
            f'<rect x="9" y="18" width="38" height="22" rx="9" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<path d="M13 21c2-2 5-3 9-3h12c4 0 7 1 9 3H13z" fill="{C_CREAM}"/>'
            f'<rect x="13.5" y="21" width="29" height="10" rx="4.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
            + kawaii_eyes(22, 34, 25.5, 1.6, blink, lx * 0.6, ly * 0.3)
            + f'<path d="M26 27c1.2 1.3 2.8 1.3 4 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
            f'<circle cx="13.5" cy="35" r="2.2" fill="{C_GOLD}"/>'
            f'<circle cx="42.5" cy="35" r="2.2" fill="{C_GOLD}"/>'
            f'<ellipse cx="18.5" cy="35" rx="2" ry="1.1" fill="{C_PINK}"/>'
            f'<ellipse cx="37.5" cy="35" rx="2" ry="1.1" fill="{C_PINK}"/>'
            f'<path d="{heart_path(28, 35, 0.55)}" fill="{C_CREAM}"/>'
            f'</g>'
            f'<circle cx="18" cy="41.5" r="4.2" fill="{C_INK}"/><circle cx="18" cy="41.5" r="1.8" fill="{C_CREAM}"/>'
            f'<circle cx="38" cy="41.5" r="4.2" fill="{C_INK}"/><circle cx="38" cy="41.5" r="1.8" fill="{C_CREAM}"/>'
        )
        return svg_wrap(56, body)

    if key == "badge_helping_wheels":
        flap = 11.0 * sin_w(u, 2.0)
        halo_s = 1.0 + 0.08 * pulse01(u, 2.0)
        xf = xf_squash(28, 30, sx, sy, bob, 8.0 * sin_w(u, 1.0))
        body = (
            f'<ellipse cx="28" cy="10" rx="{9 * halo_s:.2f}" ry="3" stroke="{C_GOLD}" stroke-width="2.5"/>'
            f'<path d="M15 26c-6-4-10-1-9 4 1 3.5 4.5 5 9 4z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round" transform="rotate({flap:.2f} 15 30)"/>'
            f'<path d="M41 26c6-4 10-1 9 4-1 3.5-4.5 5-9 4z" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round" transform="rotate({-flap:.2f} 41 30)"/>'
            f'<g transform="{xf}">'
            f'<circle cx="28" cy="30" r="13.5" stroke="{C_BLUE}" stroke-width="4.2"/>'
            f'<circle cx="28" cy="30" r="15.4" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<circle cx="28" cy="30" r="11.4" stroke="{C_INK}" stroke-width="1.3"/>'
            f'<path d="M16.5 29h23M28 32v10.5" stroke="{C_SKY}" stroke-width="3.4" stroke-linecap="round"/>'
            f'<circle cx="28" cy="30" r="6.5" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.5"/>'
            + kawaii_eyes(25.8, 30.2, 29.2, 1.2, blink, lx * 0.4, ly * 0.3)
            + f'<path d="M26.5 31.6c.9.9 2.1.9 3 0" stroke="{C_INK}" stroke-width="1.3" stroke-linecap="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "badge_traffic_light":
        xf = xf_squash(28, 30, sx, sy, bob * 0.7, 0.0)
        red_on = u < 0.33
        amb_on = 0.33 <= u < 0.66
        grn_on = u >= 0.66
        r_r = 4.3 if red_on else 3.5
        r_a = 4.3 if amb_on else 3.5
        r_g = 4.4 if grn_on else 3.5
        c_r = C_RED if red_on else "#7F1D1D"
        c_a = C_GOLD if amb_on else "#78350F"
        c_g = C_GREEN if grn_on else "#14532D"
        body = (
            twinkle_pair(u, (11, 20, 2.7, C_GOLD), (45, 18, 2.7, C_MINT))
            + f'<g transform="{xf}">'
            f'<path d="M20 11.5c2-3.5 14-3.5 16 0z" fill="{C_BLUE}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round"/>'
            f'<path d="M18 12h20" stroke="{C_GOLD}" stroke-width="2" stroke-linecap="round"/>'
            f'<rect x="17" y="13" width="22" height="33" rx="8" fill="{C_SLATE}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="28" cy="20.5" r="{r_r:.2f}" fill="{c_r}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'<circle cx="28" cy="29.5" r="{r_a:.2f}" fill="{c_a}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'<circle cx="28" cy="38.5" r="{r_g:.2f}" fill="{c_g}" stroke="{C_INK}" stroke-width="1.2"/>'
            f'<ellipse cx="19.8" cy="30" rx="1.4" ry="0.9" fill="{C_PINK}"/>'
            f'<ellipse cx="36.2" cy="30" rx="1.4" ry="0.9" fill="{C_PINK}"/>'
            f'</g>'
            f'<path d="M28 46v4M22 50h12" stroke="{C_INK}" stroke-width="4.2" stroke-linecap="round"/>'
            f'<path d="M28 46v4M22 50h12" stroke="{C_SLATE_LIGHT}" stroke-width="2.0" stroke-linecap="round"/>'
        )
        return svg_wrap(56, body)

    if key == "badge_route_compass":
        needle = 24.0 * sin_w(u, 1.0)
        xf = xf_squash(28, 30, sx, sy, bob * 0.6, 0.0)
        body = (
            twinkle_pair(u, (11, 13, 2.8, C_CYAN), (45, 38, 2.8, C_GOLD))
            + f'<g transform="{xf}">'
            f'<circle cx="28" cy="9.5" r="3.2" stroke="{C_INK}" stroke-width="1.8" fill="{C_GOLD}"/>'
            f'<circle cx="28" cy="30" r="17" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.8"/>'
            f'<circle cx="28" cy="30" r="13" fill="{C_SKY_SOFT}" stroke="{C_INK}" stroke-width="1.4"/>'
            f'<g transform="rotate({needle:.2f} 28 30)">'
            f'<polygon points="28,18.5 31.2,30 24.8,30" fill="{C_RED}" stroke="{C_INK}" stroke-width="1.2" stroke-linejoin="round"/>'
            f'<polygon points="28,41.5 31.2,30 24.8,30" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.2" stroke-linejoin="round"/>'
            f'</g>'
            f'<circle cx="28" cy="30" r="5.2" fill="{C_CREAM}" stroke="{C_INK}" stroke-width="1.4"/>'
            + kawaii_eyes(26.2, 29.8, 29.3, 1.1, blink, lx * 0.4, ly * 0.2)
            + f'<path d="M26.7 31.4c.8.8 1.8.8 2.6 0" stroke="{C_INK}" stroke-width="1.2" stroke-linecap="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "badge_traffic_whistle":
        tilt = 6.5 * sin_w(u, 2.0)
        xf = xf_squash(27, 34.5, sx, sy, -1.2 * pulse01(u, 2.0), tilt)
        p1 = 2.0 * pulse01(u, 2.0, 0.0)
        p2 = 2.0 * pulse01(u, 2.0, 0.5)
        body = (
            twinkle_pair(u, (12, 14, 2.8, C_GOLD), (10, 44, 2.2, C_CYAN))
            + f'<path d="M39 14c2.5-2.5 5-2 6 0" stroke="{C_AMBER}" stroke-width="2.4" stroke-linecap="round" transform="translate({p1:.2f} {-p1:.2f})"/>'
            f'<path d="M42 20c3-1 5 0 5.5 2" stroke="{C_CORAL}" stroke-width="2.4" stroke-linecap="round" transform="translate({p2:.2f} {-p2 * 0.5:.2f})"/>'
            f'<g transform="{xf}">'
            f'<circle cx="13" cy="33" r="3.8" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.6"/>'
            f'<path d="M27 21c-7.2 0-13 5.8-13 13s5.8 13 13 13c7 0 12.6-5.4 13-12.3l6.5-2.7V23.5H31.8C30.4 22 28.8 21 27 21z" fill="{C_SKY}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<rect x="41.5" y="23.5" width="5" height="8.5" rx="1.8" fill="{C_CORAL}" stroke="{C_INK}" stroke-width="1.5"/>'
            f'<circle cx="27" cy="34.5" r="8.2" fill="{C_WHITE}" stroke="{C_INK}" stroke-width="1.4"/>'
            + kawaii_eyes(24, 30, 33.5, 1.4, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="21.8" cy="36" rx="1.6" ry="0.9" fill="{C_BLUSH}"/>'
            f'<ellipse cx="32.2" cy="36" rx="1.6" ry="0.9" fill="{C_BLUSH}"/>'
            f'<path d="M25.2 36.4c1 1.1 2.6 1.1 3.6 0" stroke="{C_INK}" stroke-width="1.4" stroke-linecap="round"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    if key == "badge_care_shield":
        c_dy = -2.2 * sin_w(u, 1.0)
        pulse = 1.0 + 0.04 * pulse01(u, 2.0)
        xf = xf_squash(28, 33, pulse, pulse)
        body = (
            twinkle_pair(u, (10, 21, 2.7, C_GOLD), (46, 21, 2.7, C_CYAN))
            + f'<path d="M20 14.5l3-5 5 3.5 5-3.5 3 5-1.5 3.5h-13z" fill="{C_GOLD}" stroke="{C_INK}" stroke-width="1.6" stroke-linejoin="round" transform="translate(0 {c_dy:.2f})"/>'
            f'<g transform="{xf}">'
            f'<path d="M28 17l15 5v11.5c0 8.5-6.5 14.5-15 17-8.5-2.5-15-8.5-15-17V22z" fill="{C_MINT}" stroke="{C_INK}" stroke-width="1.8" stroke-linejoin="round"/>'
            f'<path d="M28 20.5l11.5 3.8v9c0 6.5-5 11.2-11.5 13.3-6.5-2.1-11.5-6.8-11.5-13.3v-9z" fill="{C_SKY_SOFT}"/>'
            + kawaii_eyes(23.5, 32.5, 29, 1.5, blink, lx * 0.5, ly * 0.3)
            + f'<ellipse cx="20.8" cy="31.8" rx="1.8" ry="1" fill="{C_PINK}"/>'
            f'<ellipse cx="35.2" cy="31.8" rx="1.8" ry="1" fill="{C_PINK}"/>'
            f'<path d="{heart_path(28, 37, 0.72)}" fill="{C_CORAL}"/>'
            f'</g>'
        )
        return svg_wrap(56, body)

    raise ValueError(f"Unknown featured badge: {key}")


# ---------------------------------------------------------------------------
# ASSET JOB REGISTRY
# ---------------------------------------------------------------------------

FEATURED_BADGE_KEYS = {
    "badge_joy_rider",
    "badge_fellowship_sprout",
    "badge_punctual_clock",
    "badge_warm_welcome",
    "moon",
    "award",
    "heart",
    "badge_helping_wheels",
    "badge_traffic_light",
    "badge_route_compass",
    "badge_traffic_whistle",
    "badge_care_shield",
}

CLASSIC_AVATAR_KEYS = [
    "car", "heart", "star", "music", "sun", "leaf",
    "coffee", "book", "church", "bike", "smile", "flower",
]

CLASSIC_BADGE_KEYS = [
    "trophy", "medal", "star", "shield", "crown", "sparkles", "flame",
    "zap", "car", "clock", "target", "sun", "coffee", "gift",
    "smile", "thumbsUp", "compass",
]

EVENT_ICON_KEYS = [
    "church", "bookOpen", "music", "flame", "heartHandshake", "sparkles", "utensils",
    "coffee", "cake", "users", "gift", "home", "trophy", "footprints", "waves",
    "bike", "tent", "mountain", "dumbbell", "target", "gamepad", "film", "camera",
    "bus", "baby", "calendar", "sun", "moon",
]


@dataclass(frozen=True)
class GifJob:
    category: str      # avatars | badges | events
    name: str          # filename stem without .gif
    render_kind: str
    variant: str
    out_size: int
    ss_size: int
    frames: int
    fps: int = 24


def build_jobs() -> list[GifJob]:
    jobs: list[GifJob] = []

    # 1. Avatars (40 files)
    for k in [
        "grace_car", "peace_dove", "sunny_cloud", "happy_bear",
        "fellowship_cat", "joy_puppy", "star_lantern", "blossom_sprout",
    ]:
        jobs.append(GifJob("avatars", k, "mascot_avatar", k, 128, 256, 48, 24))

    for c in ["default", *AVATAR_COLORS]:
        stem = "grok" if c == "default" else f"grok_{c}"
        jobs.append(GifJob("avatars", stem, "grok", c, 128, 256, 48, 24))

    for c in ["default", *AVATAR_COLORS]:
        stem = "muse" if c == "default" else f"muse_{c}"
        jobs.append(GifJob("avatars", stem, "muse", c, 128, 256, 48, 24))

    for k in CLASSIC_AVATAR_KEYS:
        jobs.append(GifJob("avatars", k, "classic_48", k, 128, 256, 36, 24))

    # 2. Badges (29 files)
    for k in sorted(FEATURED_BADGE_KEYS):
        jobs.append(GifJob("badges", k, "featured_badge", k, 128, 256, 48, 24))

    for k in CLASSIC_BADGE_KEYS:
        jobs.append(GifJob("badges", k, "classic_56", k, 128, 256, 36, 24))

    # 3. Events (28 files)
    for k in EVENT_ICON_KEYS:
        jobs.append(GifJob("events", k, "classic_48", k, 96, 192, 36, 24))

    return jobs


def render_frame_svg(job: GifJob, u: float) -> str:
    if job.render_kind == "grok":
        return render_grok_avatar(u, job.variant)
    if job.render_kind == "muse":
        return render_muse_avatar(u, job.variant)
    if job.render_kind == "mascot_avatar":
        return {
            "grace_car": render_grace_car,
            "peace_dove": render_peace_dove,
            "sunny_cloud": render_sunny_cloud,
            "happy_bear": render_happy_bear,
            "fellowship_cat": render_fellowship_cat,
            "joy_puppy": render_joy_puppy,
            "star_lantern": render_star_lantern,
            "blossom_sprout": render_blossom_sprout,
        }[job.variant](u)
    if job.render_kind == "featured_badge":
        return render_featured_badge(job.variant, u)
    if job.render_kind == "classic_56":
        return render_classic_icon(job.variant, u, vb=56)
    if job.render_kind == "classic_48":
        return render_classic_icon(job.variant, u, vb=48)
    raise ValueError(f"Unknown render_kind: {job.render_kind}")


def encode_job(job: GifJob) -> tuple[str, int]:
    out_web = WEB_ICONS_DIR / job.category / f"{job.name}.gif"
    out_and = ANDROID_ICONS_DIR / job.category / f"{job.name}.gif"
    out_web.parent.mkdir(parents=True, exist_ok=True)
    out_and.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix=f"gif_{job.category}_{job.name}_") as tmp:
        tmp_dir = Path(tmp)
        for i in range(job.frames):
            u = i / float(job.frames)
            svg_text = render_frame_svg(job, u)
            svg_path = tmp_dir / f"f_{i:03d}.svg"
            png_path = tmp_dir / f"f_{i:03d}.png"
            svg_path.write_text(svg_text, encoding="utf-8")
            subprocess.run(
                [
                    "rsvg-convert",
                    "-w",
                    str(job.ss_size),
                    "-h",
                    str(job.ss_size),
                    "-f",
                    "png",
                    "-o",
                    str(png_path),
                    str(svg_path),
                ],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
            )

        palette_path = tmp_dir / "palette.png"
        pattern = str(tmp_dir / "f_%03d.png")
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-framerate",
                str(job.fps),
                "-i",
                pattern,
                "-vf",
                f"scale={job.out_size}:{job.out_size}:flags=lanczos,palettegen=max_colors=128:reserve_transparent=1:stats_mode=diff",
                str(palette_path),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )
        subprocess.run(
            [
                "ffmpeg",
                "-y",
                "-framerate",
                str(job.fps),
                "-i",
                pattern,
                "-i",
                str(palette_path),
                "-lavfi",
                f"scale={job.out_size}:{job.out_size}:flags=lanczos [x]; [x][1:v] paletteuse=dither=sierra2_4a:alpha_threshold=128:diff_mode=rectangle",
                "-loop",
                "0",
                str(out_web),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
        )

    shutil.copy2(out_web, out_and)
    return f"{job.category}/{job.name}.gif", out_web.stat().st_size


def build_qa_sheet(out_path: Path) -> None:
    """Create a multi-frame light + dark contact sheet for visual inspection."""
    samples = [
        ("avatars/grok.gif", "avatar: grok"),
        ("avatars/grok_white.gif", "avatar: grok_white"),
        ("avatars/grok_coral.gif", "avatar: grok_coral"),
        ("avatars/muse.gif", "avatar: muse"),
        ("avatars/grace_car.gif", "avatar: grace_car"),
        ("avatars/car.gif", "avatar: car"),
        ("badges/badge_joy_rider.gif", "badge: joy_rider"),
        ("badges/moon.gif", "badge: moon"),
        ("badges/badge_traffic_light.gif", "badge: traffic_light"),
        ("badges/trophy.gif", "badge: trophy"),
        ("events/church.gif", "event: church"),
        ("events/utensils.gif", "event: utensils"),
        ("events/trophy.gif", "event: trophy"),
        ("events/cake.gif", "event: cake"),
        ("events/users.gif", "event: users"),
        ("events/calendar.gif", "event: calendar"),
    ]
    cell = 96
    pad = 12
    label_w = 180
    cols = 8  # 4 frames light + 4 frames dark
    rows = len(samples)
    w = label_w + cols * (cell + pad) + pad
    h = rows * (cell + pad) + pad + 32
    sheet = Image.new("RGB", (w, h), "#F8FAFC")
    draw = ImageDraw.Draw(sheet)
    draw.text((label_w + pad, 8), "LIGHT (#FFFFFF): t=0%, 25%, 50%, 75%", fill="#0F172A")
    draw.text((label_w + pad + 4 * (cell + pad), 8), "DARK (#111827): t=0%, 25%, 50%, 75%", fill="#0F172A")

    for r_idx, (rel, label) in enumerate(samples):
        gif_path = WEB_ICONS_DIR / rel
        y = 32 + pad + r_idx * (cell + pad)
        draw.text((12, y + cell // 2 - 6), label, fill="#0F172A")
        with Image.open(gif_path) as im:
            n_frames = getattr(im, "n_frames", 1)
            for q_idx, frac in enumerate((0.0, 0.25, 0.50, 0.75)):
                fi = min(n_frames - 1, int(frac * n_frames))
                im.seek(fi)
                fr = im.convert("RGBA").resize((cell - 12, cell - 12), Image.Resampling.LANCZOS)
                for bg_idx, bg_col in enumerate(("#FFFFFF", "#111827")):
                    x = label_w + pad + (bg_idx * 4 + q_idx) * (cell + pad)
                    tile = Image.new("RGBA", (cell, cell), bg_col)
                    tile.alpha_composite(fr, (6, 6))
                    sheet.paste(tile.convert("RGB"), (x, y))

    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--qa-sheet", type=Path, default=None)
    parser.add_argument("--workers", type=int, default=min(10, os.cpu_count() or 4))
    args = parser.parse_args()

    jobs = build_jobs()
    print(f"Generating {len(jobs)} animated Bloub-style GIF icons with {args.workers} workers...")
    total_bytes = 0
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.workers) as pool:
        for rel, size in pool.map(encode_job, jobs):
            total_bytes += size

    print(f"Done: {len(jobs)} GIFs written to web + android ({total_bytes / 1024:.1f} KiB per platform).")
    if args.qa_sheet:
        build_qa_sheet(args.qa_sheet)
        print(f"QA contact sheet saved to {args.qa_sheet}")


if __name__ == "__main__":
    main()
