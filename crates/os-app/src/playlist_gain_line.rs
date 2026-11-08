// SPDX-License-Identifier: GPL-3.0-or-later
//! An audio clip's gain line beyond dragging it (GAIN-NODE-EDIT-SPEC R1–R7): the value readout painted next to a
//! dragged or hovered node, the right-click menus of a node and of the line, their commands, and the hover hint that
//! reads the line's value at the pointer. The line's hit test and drag stay in `crate::playlist` (§9a).
//!
//! "Whole gain" everywhere here is what the engine plays at that point: the clip's static gain plus the line there.
use crate::app::{App, State};
use crate::playlist::{self as pl, Cmd, EditTarget, Handle, PGeom, PItem, Popup};
use os_model::{Clip, GainNode};
use os_ui::Rect;

/// The readout pill (R1): clip `clip`'s node's whole gain ("-6.0 dB"), painted at `pill`.
// WHY no time line: the user found two numbers next to the node noise while dragging (GAIN-SNAP-SPEC S1); the time
// stays in the node's right-click menu header and the status-bar hint.
pub struct Readout {
    pub clip: usize,
    pub value: String,
    pub pill: Rect,
}

/// Text size of the readout: the spec's floor for readable text is 11 px.
pub const VALUE_PX: u16 = 12;
/// Pill height: 3 px padding, a 14 px line, 3 px padding.
pub const PILL_H: f32 = 20.0;
pub const PILL_PAD: f32 = 6.0;
// WHY an estimate, not a measured width: the pill's rect is a pure function of the state (the view paints it and
// tests read it without a canvas). Cuprum's digits are about 6 px wide at 12 px, so 6.5 px a character leaves room;
// the view clips the text to the pill in case a glyph runs wider.
const CHAR_W: f32 = 6.5;
/// Gap between the node's disc and the pill, and the pill's margin inside the clip body.
const NODE_GAP: f32 = 8.0;
const BODY_MARGIN: f32 = 2.0;

fn whole_text(db: f32) -> String {
    format!("{:+.1} dB", db)
}

fn ms_text(ms: f32) -> String {
    format!("{:.0} ms", ms)
}

/// The pointer's time into clip `c`, held to the clip (as a press on the line places its new node).
fn pointer_ms(s: &State, g: &PGeom, c: &Clip, x: f32) -> f32 {
    pl::x_ms(s, g, c, x).clamp(0.0, pl::clip_ms(s, c))
}

fn popup(x: f32, y: f32, items: Vec<PItem>) -> Popup {
    Popup { x, y, items, sub: None, scroll: [0; 2], cascade: Vec::new() }
}

/// The node the readout belongs to: the dragged one, else the hovered one.
fn shown_node(s: &State) -> Option<(usize, usize)> {
    s.playlist.gain_drag.or(match (s.playlist.hover_clip, s.playlist.hover_handle) {
        (Some(ci), Some(Handle::GainNode(k))) => Some((ci, k)),
        _ => None,
    })
}

/// The readout to paint now, if a node is dragged or hovered (R1).
pub fn readout(s: &State, g: &PGeom) -> Option<Readout> {
    let (ci, k) = shown_node(s)?;
    let c = pl::clips(s).get(ci)?;
    let n = c.gain_line.nodes().get(k)?;
    let (nx, ny) = pl::gain_node_xy(s, g, c, k)?;
    let value = whole_text(pl::static_db(c) + n.db);
    let w = value.len() as f32 * CHAR_W + 2.0 * PILL_PAD;
    Some(Readout { clip: ci, pill: place_pill(g.body(c), g.grid, nx, ny, w), value })
}

/// The readout's value text and pill (the spec's test interface for R1).
pub fn gain_readout(s: &State, g: &PGeom) -> Option<(String, Rect)> {
    readout(s, g).map(|r| (r.value, r.pill))
}

/// Up and right of the node; flipped left of it near the clip's right edge and below it near the top; then kept
/// inside the clip's body where it fits, and always inside the grid.
fn place_pill(body: Rect, grid: Rect, nx: f32, ny: f32, w: f32) -> Rect {
    let mut x = nx + NODE_GAP;
    if x + w > body.right() - BODY_MARGIN {
        x = nx - NODE_GAP - w;
    }
    let mut y = ny - NODE_GAP - PILL_H;
    if y < body.y + BODY_MARGIN {
        y = ny + NODE_GAP;
    }
    let x = keep_inside(x, w, body.x + BODY_MARGIN, body.right() - BODY_MARGIN);
    let y = keep_inside(y, PILL_H, body.y + BODY_MARGIN, body.bottom() - BODY_MARGIN);
    Rect::new(keep_inside(x, w, grid.x, grid.right()), keep_inside(y, PILL_H, grid.y, grid.bottom()), w, PILL_H)
}

/// `v`, the start of a span `len` long, moved inside `lo..hi`; at `lo` when the span does not fit.
fn keep_inside(v: f32, len: f32, lo: f32, hi: f32) -> f32 {
    if hi - lo >= len { v.clamp(lo, hi - len) } else { lo }
}

/// Right-click on gain node `k` of clip `ci` (R2).
pub fn gain_node_menu(s: &State, ci: usize, k: usize, x: f32, y: f32) -> Popup {
    let head = pl::clips(s)
        .get(ci)
        .and_then(|c| c.gain_line.nodes().get(k).map(|n| format!("Gain node: {} @ {}", whole_text(pl::static_db(c) + n.db), ms_text(n.ms))))
        .unwrap_or_else(|| "Gain node".to_string());
    popup(x, y, vec![
        PItem::head(head),
        PItem::new("Delete node", Cmd::GainNodeDelete(ci, k)),
        PItem::new("Type value...", Cmd::GainNodeType(ci, k)),
        PItem::new("Set to 0 dB", Cmd::GainNodeZero(ci, k)),
        PItem::sep(),
        PItem::new("Reset gain line", Cmd::GainLineReset(ci)),
    ])
}

/// Right-click on clip `ci`'s gain line at the pointer (`x`, `y`) (R3).
// WHY the grid from `grid_of`: menu builders take the state only (as `clip_menu` does), and the pointer's time needs
// the grid the playlist paints, which the state determines.
pub fn gain_line_menu(s: &State, ci: usize, x: f32, y: f32) -> Popup {
    match crate::views::playlist::grid_of(s) {
        Some(grid) => line_menu(s, &PGeom::new(s, grid), ci, x, y),
        None => popup(x, y, vec![PItem::head("Gain line")]),
    }
}

fn line_menu(s: &State, g: &PGeom, ci: usize, x: f32, y: f32) -> Popup {
    let Some(c) = pl::clips(s).get(ci) else { return popup(x, y, vec![PItem::head("Gain line")]) };
    let ms = pointer_ms(s, g, c, x);
    let reset = PItem::new("Reset gain line", Cmd::GainLineReset(ci));
    popup(x, y, vec![
        PItem::head(format!("Gain line: {}", whole_text(pl::static_db(c) + c.gain_line.db_at(ms)))),
        PItem::new("Add node here", Cmd::GainLineAdd(ci, ms)),
        PItem::sep(),
        if c.gain_line.is_empty() { reset.off() } else { reset },
    ])
}

/// A right press at (`x`, `y`) on clip `ci`: opens the node or line menu when the pointer is on one, so the press
/// never reaches the tools' right-button actions (Draw deletes the clip). Whether it did.
pub fn open_menu(s: &mut State, g: &PGeom, ci: usize, x: f32, y: f32) -> bool {
    let menu = match pl::handle_at(s, g, ci, x, y) {
        Some(Handle::GainNode(k)) => gain_node_menu(s, ci, k, x, y),
        Some(Handle::GainLine) => line_menu(s, g, ci, x, y),
        _ => return false,
    };
    s.playlist.popup = Some(menu);
    true
}

/// The audio clip menu's "Reset clip gain" (R4): off when the clip is already at 0 dB with no nodes.
pub fn reset_item(c: &Clip, ci: usize) -> PItem {
    let item = PItem::new("Reset clip gain", Cmd::ClipGainReset(ci));
    if c.gain() == 1.0 && c.gain_line.is_empty() { item.off() } else { item }
}

/// "Type value...": the same box a double-click on the node opens, at the same offset from the pointer.
pub fn type_value(app: &mut App, ci: usize, k: usize) {
    let (x, y) = app.mouse;
    pl::start_edit(&mut app.state, EditTarget::GainNode(ci, k), (x + 8.0, y + 8.0));
    app.invalidate();
}

/// Runs a gain line menu command on its clip only. Each edit is one undo step, recorded only when it changes
/// something (R5).
pub fn run(s: &mut State, cmd: Cmd) -> Option<String> {
    let ci = match cmd {
        Cmd::GainNodeDelete(ci, _) | Cmd::GainNodeZero(ci, _) | Cmd::GainLineReset(ci) | Cmd::GainLineAdd(ci, _) | Cmd::ClipGainReset(ci) => ci,
        _ => return None,
    };
    let c = *pl::clips(s).get(ci)?;
    match cmd {
        Cmd::GainNodeDelete(_, k) => {
            c.gain_line.nodes().get(k)?;
            pl::snapshot_as(s, "Delete clip gain node");
            pl::arr_mut(s).clips[ci].gain_line.remove(k);
        }
        Cmd::GainNodeZero(_, k) => {
            let n = *c.gain_line.nodes().get(k)?;
            pl::snapshot_as(s, "Clip gain node to 0 dB");
            // Nodes are relative to the static gain: cancelling it makes the whole gain 0 dB there.
            pl::arr_mut(s).clips[ci].gain_line.set(k, n.ms, -pl::static_db(&c));
        }
        Cmd::GainLineReset(_) => {
            if c.gain_line.is_empty() {
                return None;
            }
            pl::snapshot_as(s, "Reset clip gain line");
            pl::arr_mut(s).clips[ci].gain_line.clear();
        }
        Cmd::GainLineAdd(_, ms) => {
            let ms = ms.clamp(0.0, pl::clip_ms(s, &c));
            let mut line = c.gain_line;
            // At the line's own value there, so the line keeps its shape.
            if line.add(GainNode { ms, db: c.gain_line.db_at(ms) }).is_none() {
                return Some("Clip gain line: no room for another node".into());
            }
            pl::snapshot_as(s, "Add clip gain node");
            pl::arr_mut(s).clips[ci].gain_line = line;
        }
        Cmd::ClipGainReset(_) => {
            if c.gain() == 1.0 && c.gain_line.is_empty() {
                return None;
            }
            pl::snapshot_as(s, "Reset clip gain");
            let clip = &mut pl::arr_mut(s).clips[ci];
            clip.set_gain(1.0);
            clip.gain_line.clear();
        }
        _ => {}
    }
    s.piano.dirty = true;
    None
}

/// The line's hover hint (R7), at the whole gain `db` under the pointer.
pub fn line_hint(db: f32) -> String {
    format!("Clip gain line: {} (click to add a node, right-click for options)", whole_text(db))
}

/// The node's hover hint (R7).
pub fn node_hint(c: &Clip, n: GainNode) -> String {
    format!(
        "Clip gain node: {} at {} (drag to move, Alt: no snap, Shift: fine, double-click to type, right-click for options)",
        whole_text(pl::static_db(c) + n.db),
        ms_text(n.ms)
    )
}

/// The hover hint of handle `h` of clip `ci` with the pointer at `x`: the line's reads its value there.
pub fn hover_hint(s: &State, g: &PGeom, ci: usize, h: Handle, x: f32) -> String {
    match (pl::clips(s).get(ci), h) {
        (Some(c), Handle::GainLine) => line_hint(pl::static_db(c) + c.gain_line.db_at(pointer_ms(s, g, c, x))),
        (Some(c), _) => pl::handle_hint(c, h),
        (None, _) => String::new(),
    }
}
