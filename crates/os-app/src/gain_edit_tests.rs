//! Acceptance tests for the clip gain line edit change (GAIN-NODE-EDIT-SPEC.md, R1–R7)
//! and the follow-up value-only readout + time snapping (GAIN-SNAP-SPEC.md, S1–S4).
//!
//! Written from the spec and the public interface only (no implementation source was read).
//! These are the contract: the implementer may not edit them.
//!
//! Nothing is mocked: every test drives the real `App` (mouse events + paint into a real
//! off-screen `Canvas`) and the real undo history. The only fixture is a 1000 ms stereo wav
//! written to the temp dir. 1000 ms clip, 800 px/bar zoom -> ~433 px wide, rows 160 px tall.
//!
//! Conventions used below:
//! - "whole gain" of a node = `pl::static_db(&clip) + node.db` (spec, top).
//! - value text = `format!("{:+.1} dB", whole)`, time text = `format!("{:.0} ms", ms)`.
//! - Undo must restore nodes and static gain EXACTLY, so snapshots compare f32 bit patterns.

use crate::app::App;
use crate::playlist as pl;
use os_ui::{Canvas, Rect};

// Fixture length: 96000 interleaved stereo samples = 48000 frames at 48 kHz = 1000 ms.
const CLIP_MS: f32 = 1000.0;

// ---------- helpers ----------

fn fmt_db(v: f32) -> String {
    format!("{:+.1} dB", v)
}

fn fmt_ms(ms: f32) -> String {
    format!("{:.0} ms", ms)
}

fn db_to_lin(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

fn write_wav(name: &str) -> std::path::PathBuf {
    let wav = std::env::temp_dir().join(format!("os-app-gain-edit-{}-{}.wav", std::process::id(), name));
    os_engine::wav::write_f32_stereo(&mut std::fs::File::create(&wav).unwrap(), 48000, &[0.5; 96000])
        .unwrap();
    wav
}

/// App with the playlist on top, tall rows, one 1000 ms audio clip at track 0 / tick 0.
/// Returns (canvas, app, clip index).
fn setup(name: &str) -> Option<(Canvas, App, usize)> {
    setup_at(name, 0)
}

/// Same as `setup`, with the clip placed at song tick `tick` (S2: off-grid clip positions).
fn setup_at(name: &str, tick: u32) -> Option<(Canvas, App, usize)> {
    let Ok(mut c) = Canvas::new(1920, 1080, 1.0) else { return None };
    let mut app = App::new(None);
    app.state.project = crate::app::test_project();
    app.state.playlist.reset();
    app.state.z = vec![crate::app::Win::Playlist];
    app.paint(&mut c, None);
    // WHY 800: at 400 px/bar the 2000 ms fixture is only ~217 px wide; the pill-inside-clip
    // and right-edge-flip checks need room for a label plus margin.
    app.state.playlist_bar_w = 800.0;
    // WHY: dB steps per pixel are only meaningful on a tall clip (spec note: body.h >= 60).
    app.state.playlist.row_h = 160.0;
    let wav = write_wav(name);
    pl::add_audio_clip(&mut app.state, &wav.to_string_lossy(), 0, tick).expect("audio clip added");
    let ci = pl::clips(&app.state).len() - 1;
    assert_eq!(pl::clips(&app.state)[ci].position, tick, "test precondition: clip placed at tick {tick}");
    app.state.playlist.sel.clear();
    app.paint(&mut c, None);
    let g = geom(&app);
    let body = g.body(&pl::clips(&app.state)[ci]);
    assert!(body.h >= 60.0, "test precondition: clip body must be >= 60 px tall, got {}", body.h);
    assert!(body.w >= 300.0, "test precondition: clip body must be >= 300 px wide, got {}", body.w);
    Some((c, app, ci))
}

fn geom(app: &App) -> pl::PGeom {
    pl::PGeom::new(&app.state, app.pl_grid().expect("playlist grid visible"))
}

fn clip(app: &App, ci: usize) -> os_model::Clip {
    let cs = pl::clips(&app.state);
    assert!(ci < cs.len(), "clip {ci} is gone ({} clips left): a right-click on a gain node/line must never delete the clip", cs.len());
    cs[ci]
}

fn set_static_db(app: &mut App, ci: usize, db: f32) {
    pl::arr_mut(&mut app.state).clips[ci].set_gain(db_to_lin(db));
}

fn clear_nodes(app: &mut App, ci: usize) {
    pl::arr_mut(&mut app.state).clips[ci].gain_line = Default::default();
}

fn add_node(app: &mut App, ci: usize, ms: f32, db: f32) -> usize {
    pl::arr_mut(&mut app.state).clips[ci]
        .gain_line
        .add(os_model::GainNode { ms, db })
        .expect("node added")
}

fn nodes(app: &App, ci: usize) -> Vec<(f32, f32)> {
    clip(app, ci).gain_line.nodes().iter().map(|n| (n.ms, n.db)).collect()
}

fn whole(app: &App, ci: usize, k: usize) -> f32 {
    let c = clip(app, ci);
    pl::static_db(&c) + c.gain_line.nodes()[k].db
}

/// Bit-exact snapshot of a clip's gain state (nodes + static gain) for undo checks.
fn snap(app: &App, ci: usize) -> (Vec<(u32, u32)>, u32) {
    let c = clip(app, ci);
    let ns = c.gain_line.nodes().iter().map(|n| (n.ms.to_bits(), n.db.to_bits())).collect();
    (ns, c.gain().to_bits())
}

fn node_xy(app: &App, ci: usize, k: usize) -> (f32, f32) {
    let g = geom(app);
    pl::gain_node_xy(&app.state, &g, &clip(app, ci), k).expect("node has a screen position")
}

fn hover(c: &mut Canvas, app: &mut App, x: f32, y: f32) {
    app.mouse_move(x, y);
    app.paint(c, None);
}

fn right_click(c: &mut Canvas, app: &mut App, x: f32, y: f32) {
    hover(c, app, x, y);
    app.mouse_down_right();
    app.mouse_up();
    app.paint(c, None);
}

fn click(c: &mut Canvas, app: &mut App, x: f32, y: f32) {
    hover(c, app, x, y);
    app.mouse_down();
    app.mouse_up();
    app.paint(c, None);
}

/// Popup rendered as a list of strings; separators become "---".
fn popup_labels(p: &pl::Popup) -> Vec<String> {
    p.items
        .iter()
        .map(|it| if it.sep { "---".to_string() } else { it.label.to_string() })
        .collect()
}

fn item<'a>(p: &'a pl::Popup, label: &str) -> &'a pl::PItem {
    p.items
        .iter()
        .find(|it| !it.sep && it.label == label)
        .unwrap_or_else(|| panic!("popup has no item {label:?}; items: {:?}", popup_labels(p)))
}

/// Pick a popup item the way the user does: it must be enabled, then its command runs.
fn choose(app: &mut App, label: &str) {
    let p = app.state.playlist.popup.clone().expect("a popup is open");
    let it = item(&p, label);
    assert!(it.enabled, "item {label:?} must be enabled");
    let cmd = it.cmd.clone();
    app.state.playlist.popup = None;
    pl::run(app, cmd);
}

/// Runs `f`, asserts it was exactly one undo step, undoes it, asserts exact restore.
fn assert_one_undo_step(app: &mut App, ci: usize, f: impl FnOnce(&mut App)) {
    let before = snap(app, ci);
    let h0 = app.state.history.len();
    f(app);
    assert_ne!(snap(app, ci), before, "the edit must change the clip's gain state");
    assert_eq!(app.state.history.len(), h0 + 1, "the edit must be exactly one undo step");
    assert!(crate::history::undo(&mut app.state).is_some(), "undo must have something to undo");
    assert_eq!(snap(app, ci), before, "one undo must restore nodes and static gain exactly");
}

const TOOLS: [pl::PlTool; 4] = [pl::PlTool::Draw, pl::PlTool::Select, pl::PlTool::Paint, pl::PlTool::Slice];

// ---------- R1: live value readout ----------

#[test]
fn gain_edit_r1_no_readout_without_drag_or_hover() {
    let Some((_c, app, _ci)) = setup("r1-none") else { return };
    let g = geom(&app);
    assert!(pl::gain_readout(&app.state, &g).is_none(), "no node dragged or hovered -> None");
}

#[test]
fn gain_edit_r1_readout_follows_every_drag_frame_with_whole_gain() {
    let Some((mut c, mut app, ci)) = setup("r1-drag") else { return };
    // Static -6 dB so a readout showing node.db alone (not the whole gain) is caught.
    set_static_db(&mut app, ci, -6.0);
    app.paint(&mut c, None);
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.5;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    hover(&mut c, &mut app, x, y);
    assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainLine));
    app.mouse_down();
    app.paint(&mut c, None);
    let mut seen = std::collections::BTreeSet::new();
    for k in 1..=8 {
        let f = k as f32 / 8.0;
        app.mouse_move(x, y + 40.0 * f);
        app.paint(&mut c, None);
        assert_eq!(nodes(&app, ci).len(), 1, "press on the line adds exactly one node");
        let want = fmt_db(whole(&app, ci, 0));
        let (text, _) = pl::gain_readout(&app.state, &geom(&app))
            .unwrap_or_else(|| panic!("frame {k}: readout must be shown while dragging"));
        assert!(text.starts_with(&want), "frame {k}: readout {text:?} must show whole gain {want:?}");
        // S1: the time line is gone from the pill.
        assert!(!text.contains("ms"), "frame {k}: readout {text:?} must not show a time (S1)");
        seen.insert(want);
    }
    assert!(seen.len() >= 4, "dragging 40 px must move the value through several steps: {seen:?}");
    app.mouse_up();
    // Pointer leaves the node: readout disappears.
    hover(&mut c, &mut app, 2.0, 2.0);
    assert!(pl::gain_readout(&app.state, &geom(&app)).is_none(), "readout must go after release + leave");
}

#[test]
fn gain_edit_r1_readout_on_hover_shows_whole_gain() {
    let Some((mut c, mut app, ci)) = setup("r1-hover") else { return };
    set_static_db(&mut app, ci, -6.0);
    add_node(&mut app, ci, 500.0, -6.0);
    app.paint(&mut c, None);
    let (nx, ny) = node_xy(&app, ci, 0);
    hover(&mut c, &mut app, nx, ny);
    assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainNode(0)));
    let (text, _) = pl::gain_readout(&app.state, &geom(&app)).expect("hovered node shows a readout");
    let want = fmt_db(whole(&app, ci, 0));
    assert_eq!(want, "-12.0 dB", "fixture sanity: -6 static + -6 node");
    assert!(text.starts_with(&want), "readout {text:?} must start with {want:?}");
    assert!(!text.contains("ms"), "readout {text:?} must not show a time (S1)");
}

/// Pill must be readable (>= 11 px text -> pill at least 11 px tall), inside the playlist
/// grid, and inside the clip horizontally; near the clip's right edge it flips left of the node.
#[test]
fn gain_edit_r1_readout_pill_geometry() {
    let Some((mut c, mut app, ci)) = setup("r1-geom") else { return };
    for ms in [20.0, 1000.0, CLIP_MS - 20.0] {
        clear_nodes(&mut app, ci);
        add_node(&mut app, ci, ms, -3.0);
        app.paint(&mut c, None);
        let (nx, ny) = node_xy(&app, ci, 0);
        hover(&mut c, &mut app, nx, ny);
        let g = geom(&app);
        let body = g.body(&clip(&app, ci));
        let grid = app.pl_grid().unwrap();
        let (_, r): (String, Rect) = pl::gain_readout(&app.state, &g).expect("readout on hover");
        assert!(r.h >= 11.0, "ms={ms}: pill {r:?} too short for >= 11 px text");
        assert!(r.w > 0.0, "ms={ms}: pill has no width");
        // 0.5 px: allow sub-pixel rounding of the pill edges.
        assert!(r.x >= grid.x - 0.5 && r.right() <= grid.right() + 0.5, "ms={ms}: pill {r:?} outside grid {grid:?}");
        assert!(r.y >= grid.y - 0.5 && r.bottom() <= grid.bottom() + 0.5, "ms={ms}: pill {r:?} outside grid {grid:?}");
        assert!(r.x >= body.x - 0.5 && r.right() <= body.right() + 0.5, "ms={ms}: pill {r:?} not inside clip {body:?}");
        if ms > CLIP_MS * 0.9 {
            assert!(r.right() <= nx + 0.5, "ms={ms}: near the right edge the pill must flip left of the node (x={nx}), got {r:?}");
        }
    }
}

// ---------- R2: right-click a node ----------

/// Static -6 dB; nodes at 250/500/750 ms with -3/+4.5/-9 dB.
fn three_nodes(c: &mut Canvas, app: &mut App, ci: usize) {
    set_static_db(app, ci, -6.0);
    add_node(app, ci, 250.0, -3.0);
    add_node(app, ci, 500.0, 4.5);
    add_node(app, ci, 750.0, -9.0);
    app.paint(c, None);
}

#[test]
fn gain_edit_r2_right_click_node_opens_menu_in_every_tool_and_keeps_clip() {
    for tool in TOOLS {
        let Some((mut c, mut app, ci)) = setup("r2-tools") else { return };
        three_nodes(&mut c, &mut app, ci);
        app.state.playlist.tool = tool;
        let n0 = pl::clips(&app.state).len();
        let before_nodes = nodes(&app, ci);
        let body0 = geom(&app).body(&clip(&app, ci));
        let (nx, ny) = node_xy(&app, ci, 1);
        right_click(&mut c, &mut app, nx, ny);
        assert_eq!(pl::clips(&app.state).len(), n0, "{tool:?}: right-click on a node must not delete the clip");
        assert_eq!(nodes(&app, ci), before_nodes, "{tool:?}: right-click must not edit nodes");
        let body1 = geom(&app).body(&clip(&app, ci));
        assert_eq!((body0.x, body0.y, body0.w), (body1.x, body1.y, body1.w), "{tool:?}: clip must not move");
        let p = app.state.playlist.popup.clone().unwrap_or_else(|| panic!("{tool:?}: node menu must open"));
        let head = format!("Gain node: {} @ {}", fmt_db(whole(&app, ci, 1)), fmt_ms(500.0));
        assert_eq!(head, "Gain node: -1.5 dB @ 500 ms", "fixture sanity");
        assert_eq!(
            popup_labels(&p),
            vec![head.as_str(), "Delete node", "Type value...", "Set to 0 dB", "---", "Reset gain line"],
            "{tool:?}: node menu items/order"
        );
        assert!(p.items[0].head, "{tool:?}: first item is a header");
        assert_eq!(item(&p, "Delete node").cmd, pl::Cmd::GainNodeDelete(ci, 1));
        assert_eq!(item(&p, "Type value...").cmd, pl::Cmd::GainNodeType(ci, 1));
        assert_eq!(item(&p, "Set to 0 dB").cmd, pl::Cmd::GainNodeZero(ci, 1));
        assert_eq!(item(&p, "Reset gain line").cmd, pl::Cmd::GainLineReset(ci));
    }
}

#[test]
fn gain_edit_r2_gain_node_menu_fn_matches_spec() {
    let Some((mut c, mut app, ci)) = setup("r2-fn") else { return };
    three_nodes(&mut c, &mut app, ci);
    let p = pl::gain_node_menu(&app.state, ci, 2, 100.0, 100.0);
    let head = format!("Gain node: {} @ {}", fmt_db(whole(&app, ci, 2)), fmt_ms(750.0));
    assert_eq!(
        popup_labels(&p),
        vec![head.as_str(), "Delete node", "Type value...", "Set to 0 dB", "---", "Reset gain line"]
    );
    assert_eq!(item(&p, "Delete node").cmd, pl::Cmd::GainNodeDelete(ci, 2));
}

#[test]
fn gain_edit_r2_delete_node_removes_only_that_node_and_undoes() {
    let Some((mut c, mut app, ci)) = setup("r2-del") else { return };
    three_nodes(&mut c, &mut app, ci);
    let before = nodes(&app, ci);
    let (nx, ny) = node_xy(&app, ci, 1);
    right_click(&mut c, &mut app, nx, ny);
    let g0 = clip(&app, ci).gain().to_bits();
    assert_one_undo_step(&mut app, ci, |app| {
        choose(app, "Delete node");
        assert_eq!(nodes(app, ci), vec![before[0], before[2]], "only the clicked node is removed");
        assert_eq!(clip(app, ci).gain().to_bits(), g0, "static gain untouched");
    });
}

#[test]
fn gain_edit_r2_type_value_opens_node_text_box() {
    let Some((mut c, mut app, ci)) = setup("r2-type") else { return };
    three_nodes(&mut c, &mut app, ci);
    let (nx, ny) = node_xy(&app, ci, 2);
    right_click(&mut c, &mut app, nx, ny);
    choose(&mut app, "Type value...");
    let ed = app.state.playlist.edit.as_ref().expect("text box opens");
    assert!(
        matches!(ed.target, pl::EditTarget::GainNode(a, b) if a == ci && b == 2),
        "edit target must be GainNode({ci}, 2)"
    );
}

#[test]
fn gain_edit_r2_set_to_zero_makes_whole_gain_zero_keeps_time_and_undoes() {
    for static_db in [-6.0f32, 0.0, 7.5, -20.0] {
        let Some((mut c, mut app, ci)) = setup("r2-zero") else { return };
        set_static_db(&mut app, ci, static_db);
        add_node(&mut app, ci, 350.0, -3.0);
        add_node(&mut app, ci, 650.0, 2.0);
        app.paint(&mut c, None);
        let (nx, ny) = node_xy(&app, ci, 0);
        right_click(&mut c, &mut app, nx, ny);
        let other = nodes(&app, ci)[1];
        assert_one_undo_step(&mut app, ci, |app| {
            choose(app, "Set to 0 dB");
            let ns = nodes(app, ci);
            assert_eq!(ns.len(), 2, "static {static_db}: node count unchanged");
            assert_eq!(ns[0].0, 350.0, "static {static_db}: time unchanged");
            // 1e-3 dB: only f32 rounding of static_db (computed from linear gain) is allowed.
            let w = whole(app, ci, 0);
            assert!(w.abs() < 1e-3, "static {static_db}: whole gain must be 0, got {w}");
            assert_eq!(ns[1], other, "static {static_db}: other node untouched");
        });
    }
}

#[test]
fn gain_edit_r2_reset_gain_line_from_node_menu_removes_all_and_undoes() {
    let Some((mut c, mut app, ci)) = setup("r2-reset") else { return };
    three_nodes(&mut c, &mut app, ci);
    let (nx, ny) = node_xy(&app, ci, 0);
    right_click(&mut c, &mut app, nx, ny);
    let g0 = clip(&app, ci).gain().to_bits();
    assert_one_undo_step(&mut app, ci, |app| {
        choose(app, "Reset gain line");
        assert!(clip(app, ci).gain_line.is_empty(), "all nodes removed");
        assert_eq!(clip(app, ci).gain().to_bits(), g0, "line flat at the static gain (static unchanged)");
    });
    assert_eq!(nodes(&app, ci).len(), 3, "undo brings back all 3");
}

// ---------- R3: right-click the line ----------

/// Static -6 dB; sloped line from -12 dB @ 100 ms to +6 dB @ 900 ms.
fn sloped(c: &mut Canvas, app: &mut App, ci: usize) {
    set_static_db(app, ci, -6.0);
    add_node(app, ci, 100.0, -12.0);
    add_node(app, ci, 900.0, 6.0);
    app.paint(c, None);
}

#[test]
fn gain_edit_r3_right_click_line_opens_line_menu_in_every_tool() {
    for tool in TOOLS {
        let Some((mut c, mut app, ci)) = setup("r3-tools") else { return };
        sloped(&mut c, &mut app, ci);
        app.state.playlist.tool = tool;
        let g = geom(&app);
        let cl = clip(&app, ci);
        let body = g.body(&cl);
        let x = body.x + body.w * 0.37;
        let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
        let ms = pl::x_ms(&app.state, &g, &cl, x);
        let value = pl::static_db(&cl) + cl.gain_line.db_at(ms);
        let n0 = pl::clips(&app.state).len();
        let before = nodes(&app, ci);
        hover(&mut c, &mut app, x, y);
        assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainLine), "{tool:?}: pointer on the line");
        right_click(&mut c, &mut app, x, y);
        assert_eq!(pl::clips(&app.state).len(), n0, "{tool:?}: right-click on the line must not delete the clip");
        assert_eq!(nodes(&app, ci), before, "{tool:?}: right-click alone must not add a node");
        let p = app.state.playlist.popup.clone().unwrap_or_else(|| panic!("{tool:?}: line menu must open"));
        let head = format!("Gain line: {}", fmt_db(value));
        let labels: Vec<String> = popup_labels(&p).into_iter().filter(|l| l != "---").collect();
        assert_eq!(labels, vec![head.as_str(), "Add node here", "Reset gain line"], "{tool:?}: line menu");
        assert!(p.items[0].head, "{tool:?}: first item is a header");
        assert!(item(&p, "Reset gain line").enabled, "{tool:?}: reset enabled when nodes exist");
        assert_eq!(item(&p, "Reset gain line").cmd, pl::Cmd::GainLineReset(ci));
        match item(&p, "Add node here").cmd {
            pl::Cmd::GainLineAdd(a, m) => {
                assert_eq!(a, ci);
                // One pixel of pointer time: the menu may quantise to the pixel it was opened at.
                let px_ms = (pl::x_ms(&app.state, &g, &cl, x + 1.0) - ms).abs();
                assert!((m - ms).abs() <= px_ms + 1e-3, "{tool:?}: add at pointer time {ms}, got {m}");
            }
            ref other => panic!("{tool:?}: Add node here must be GainLineAdd, got {other:?}"),
        }
    }
}

#[test]
fn gain_edit_r3_reset_disabled_on_empty_line_and_header_is_static() {
    let Some((mut c, mut app, ci)) = setup("r3-empty") else { return };
    set_static_db(&mut app, ci, -6.0);
    app.paint(&mut c, None);
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.5;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    right_click(&mut c, &mut app, x, y);
    let p = app.state.playlist.popup.clone().expect("line menu opens on an empty (flat) line too");
    assert_eq!(p.items[0].label, format!("Gain line: {}", fmt_db(pl::static_db(&cl))));
    assert!(!item(&p, "Reset gain line").enabled, "reset disabled when the clip has no nodes");
    let p2 = pl::gain_line_menu(&app.state, ci, x, y);
    assert!(!item(&p2, "Reset gain line").enabled, "gain_line_menu: reset disabled with no nodes");
}

#[test]
fn gain_edit_r3_add_node_here_does_not_move_the_line_and_undoes() {
    let Some((mut c, mut app, ci)) = setup("r3-add") else { return };
    sloped(&mut c, &mut app, ci);
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.61;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    let ms = pl::x_ms(&app.state, &g, &cl, x);
    let probe: Vec<f32> = (0..=100).map(|i| CLIP_MS * i as f32 / 100.0).collect();
    let line0: Vec<f32> = probe.iter().map(|&t| cl.gain_line.db_at(t)).collect();
    right_click(&mut c, &mut app, x, y);
    assert_one_undo_step(&mut app, ci, |app| {
        choose(app, "Add node here");
        let after = clip(app, ci);
        let ns = nodes(app, ci);
        assert_eq!(ns.len(), 3, "one node added");
        let px_ms = (pl::x_ms(&app.state, &g, &cl, x + 1.0) - ms).abs();
        assert!(ns.iter().any(|n| (n.0 - ms).abs() <= px_ms + 1e-3), "new node at pointer time {ms}: {ns:?}");
        for (t, d0) in probe.iter().zip(&line0) {
            // 0.05 dB: inserting a node at the line's own value must not reshape it; this
            // allows f32 interpolation rounding only (well below the 0.1 dB display step).
            let d1 = after.gain_line.db_at(*t);
            assert!((d1 - d0).abs() < 0.05, "line moved at {t} ms: {d0} -> {d1}");
        }
        assert_eq!(after.gain().to_bits(), cl.gain().to_bits(), "static gain untouched");
    });
}

#[test]
fn gain_edit_r3_reset_gain_line_from_line_menu_undoes_all() {
    let Some((mut c, mut app, ci)) = setup("r3-reset") else { return };
    three_nodes(&mut c, &mut app, ci);
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.37; // between node 0 (250 ms) and node 1 (500 ms)
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    right_click(&mut c, &mut app, x, y);
    assert!(
        app.state.playlist.popup.as_ref().is_some_and(|p| p.items[0].label.starts_with("Gain line: ")),
        "line menu opens"
    );
    assert_one_undo_step(&mut app, ci, |app| {
        choose(app, "Reset gain line");
        assert!(clip(app, ci).gain_line.is_empty());
    });
    assert_eq!(nodes(&app, ci).len(), 3);
}

// ---------- R4: reset from the clip menu ----------

#[test]
fn gain_edit_r4_clip_menu_reset_enabled_only_when_not_default() {
    let cases: [(f32, bool, bool); 4] = [(0.0, false, false), (-6.0, false, true), (0.0, true, true), (3.0, true, true)];
    for (static_db, with_nodes, want_enabled) in cases {
        let Some((mut c, mut app, ci)) = setup("r4-en") else { return };
        if static_db != 0.0 {
            set_static_db(&mut app, ci, static_db);
        }
        if with_nodes {
            add_node(&mut app, ci, 500.0, -4.0);
        }
        app.paint(&mut c, None);
        let p = pl::clip_menu(&app.state, ci, 100.0, 100.0);
        let it = item(&p, "Reset clip gain");
        assert_eq!(it.enabled, want_enabled, "static {static_db} dB, nodes {with_nodes}");
        assert_eq!(it.cmd, pl::Cmd::ClipGainReset(ci));
    }
}

#[test]
fn gain_edit_r4_reset_clip_gain_zeroes_static_and_clears_nodes_and_undoes() {
    let Some((mut c, mut app, ci)) = setup("r4-run") else { return };
    three_nodes(&mut c, &mut app, ci);
    app.state.playlist.popup = Some(pl::clip_menu(&app.state, ci, 100.0, 100.0));
    assert_one_undo_step(&mut app, ci, |app| {
        choose(app, "Reset clip gain");
        let cl = clip(app, ci);
        assert!(cl.gain_line.is_empty(), "nodes removed");
        // 1e-6 dB: static must be unity (0 dB) up to f32 log rounding.
        assert!(pl::static_db(&cl).abs() < 1e-6, "static gain 0 dB, got {}", pl::static_db(&cl));
    });
}

/// Commands carry a clip index; they must touch that clip only (the "wrong tenant" case).
#[test]
fn gain_edit_r4_commands_touch_only_their_clip() {
    let Some((mut c, mut app, ci)) = setup("r4-other-a") else { return };
    let wav = write_wav("r4-other-b");
    pl::add_audio_clip(&mut app.state, &wav.to_string_lossy(), 1, 0).expect("second clip added");
    let cj = pl::clips(&app.state).len() - 1;
    assert_ne!(ci, cj);
    for i in [ci, cj] {
        set_static_db(&mut app, i, -6.0);
        add_node(&mut app, i, 250.0, -3.0);
        add_node(&mut app, i, 750.0, 3.0);
    }
    app.paint(&mut c, None);
    let other = snap(&app, cj);
    for cmd in [
        pl::Cmd::GainNodeZero(ci, 0),
        pl::Cmd::GainLineAdd(ci, 500.0),
        pl::Cmd::GainNodeDelete(ci, 0),
        pl::Cmd::GainLineReset(ci),
        pl::Cmd::ClipGainReset(ci),
    ] {
        pl::run(&mut app, cmd.clone());
        assert_eq!(snap(&app, cj), other, "{cmd:?} must not touch clip {cj}");
    }
    assert!(clip(&app, ci).gain_line.is_empty(), "target clip was reset");
    assert!(pl::static_db(&clip(&app, ci)).abs() < 1e-6, "target clip static gain reset");
}

// ---------- R5: undo property over generated states ----------

/// Every edit command is exactly one undo step and one undo restores nodes + static gain
/// bit-exactly, over generated static gains, node sets and target nodes.
#[test]
fn gain_edit_r5_property_each_edit_is_one_exact_undo_step() {
    let Some((mut c, mut app, ci)) = setup("r5-prop") else { return };
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        // xorshift64: deterministic, no new dependency.
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut ran = [0usize; 5];
    for case in 0..100 {
        clear_nodes(&mut app, ci);
        let sdb = -24.0 + 36.0 * rnd();
        set_static_db(&mut app, ci, sdb);
        let n = 1 + (rnd() * 6.0) as usize;
        for _ in 0..n {
            let ms = (CLIP_MS * rnd()).round();
            let db = pl::GAIN_LINE_MIN_DB + (pl::GAIN_LINE_MAX_DB - pl::GAIN_LINE_MIN_DB) * rnd();
            let _ = pl::arr_mut(&mut app.state).clips[ci].gain_line.add(os_model::GainNode { ms, db });
        }
        let len = nodes(&app, ci).len();
        if len == 0 {
            continue;
        }
        let k = ((rnd() * len as f32) as usize).min(len - 1);
        let add_ms = (CLIP_MS * rnd()).round() + 0.5; // off any existing integer-ms node
        let op = case % 5;
        let cmd = match op {
            0 => pl::Cmd::GainNodeDelete(ci, k),
            1 => pl::Cmd::GainNodeZero(ci, k),
            2 => pl::Cmd::GainLineReset(ci),
            3 => pl::Cmd::GainLineAdd(ci, add_ms),
            _ => pl::Cmd::ClipGainReset(ci),
        };
        if op == 1 && whole(&app, ci, k).abs() < 1e-3 {
            continue; // already at 0 dB: nothing to change
        }
        app.paint(&mut c, None);
        let before = snap(&app, ci);
        // WHY done() and not len(): len() also counts the redo tail, and each case
        // ends with an undo, so the next push truncates that tail and len() stays put.
        // done() is the number of steps undo can reach, which is what R5 is about.
        let d0 = app.state.history.done();
        pl::run(&mut app, cmd.clone());
        assert_ne!(snap(&app, ci), before, "case {case} {cmd:?}: must change state");
        assert_eq!(app.state.history.done(), d0 + 1, "case {case} {cmd:?}: exactly one undo step");
        assert!(crate::history::undo(&mut app.state).is_some());
        assert_eq!(app.state.history.done(), d0, "case {case} {cmd:?}: one undo consumes the one step");
        assert_eq!(snap(&app, ci), before, "case {case} {cmd:?}: undo must restore exactly");
        ran[op] += 1;
    }
    assert!(ran.iter().all(|&n| n >= 10), "generator must exercise every op: {ran:?}");
}

// ---------- R6: existing gestures keep working ----------

#[test]
fn gain_edit_r6_left_press_on_line_adds_and_drags_a_node() {
    let Some((mut c, mut app, ci)) = setup("r6-press") else { return };
    let n0 = pl::clips(&app.state).len();
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.5;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    hover(&mut c, &mut app, x, y);
    app.mouse_down();
    for k in 1..=8 {
        app.mouse_move(x, y + 5.0 * k as f32);
    }
    app.mouse_up();
    let ns = nodes(&app, ci);
    assert_eq!(ns.len(), 1, "press on line adds one node");
    assert!(ns[0].1 < -1.0, "dragging down lowers the node, got {} dB", ns[0].1);
    assert_eq!(pl::clips(&app.state).len(), n0, "clip kept");
}

#[test]
fn gain_edit_r6_alt_click_node_deletes_and_alt_click_line_clears() {
    let Some((mut c, mut app, ci)) = setup("r6-alt") else { return };
    three_nodes(&mut c, &mut app, ci);
    app.alt = true;
    let (nx, ny) = node_xy(&app, ci, 1);
    let before = nodes(&app, ci);
    click(&mut c, &mut app, nx, ny);
    assert_eq!(nodes(&app, ci), vec![before[0], before[2]], "alt-click deletes that node");
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.6;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    click(&mut c, &mut app, x, y);
    app.alt = false;
    assert!(clip(&app, ci).gain_line.is_empty(), "alt-click on the line clears it");
}

#[test]
fn gain_edit_r6_double_click_node_opens_value_box() {
    let Some((mut c, mut app, ci)) = setup("r6-dbl") else { return };
    three_nodes(&mut c, &mut app, ci);
    let (nx, ny) = node_xy(&app, ci, 0);
    click(&mut c, &mut app, nx, ny);
    click(&mut c, &mut app, nx, ny);
    let ed = app.state.playlist.edit.as_ref().expect("double-click opens the value box");
    assert!(matches!(ed.target, pl::EditTarget::GainNode(a, 0) if a == ci));
}

#[test]
fn gain_edit_r6_draw_tool_right_click_away_from_line_still_deletes_clip() {
    let Some((mut c, mut app, ci)) = setup("r6-del") else { return };
    three_nodes(&mut c, &mut app, ci);
    app.state.playlist.tool = pl::PlTool::Draw;
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.25;
    let ly = pl::gain_line_y_at(&app.state, &g, &cl, x);
    // Farther of two points in the lower body, at least 25 px off the line.
    let y = [body.y + body.h * 0.55, body.bottom() - 8.0]
        .into_iter()
        .max_by(|a, b| (a - ly).abs().total_cmp(&(b - ly).abs()))
        .unwrap();
    assert!((y - ly).abs() >= 25.0, "precondition: point is away from the line");
    hover(&mut c, &mut app, x, y);
    assert!(
        !matches!(app.state.playlist.hover_handle, Some(pl::Handle::GainLine) | Some(pl::Handle::GainNode(_))),
        "precondition: not on the gain line or a node"
    );
    let n0 = pl::clips(&app.state).len();
    right_click(&mut c, &mut app, x, y);
    assert_eq!(pl::clips(&app.state).len(), n0 - 1, "Draw tool right-click on the body still deletes the clip");
}

// ---------- R7: hints ----------

#[test]
fn gain_edit_r7_node_hint() {
    let Some((mut c, mut app, ci)) = setup("r7-node") else { return };
    three_nodes(&mut c, &mut app, ci);
    let (nx, ny) = node_xy(&app, ci, 1);
    hover(&mut c, &mut app, nx, ny);
    let want = format!(
        // S4 supersedes R7's node hint text (adds the Alt / Shift modifiers).
        "Clip gain node: {} at {} (drag to move, Alt: no snap, Shift: fine, double-click to type, right-click for options)",
        fmt_db(whole(&app, ci, 1)),
        fmt_ms(500.0)
    );
    assert_eq!(app.hint.as_ref(), want);
}

#[test]
fn gain_edit_r7_line_hint() {
    let Some((mut c, mut app, ci)) = setup("r7-line") else { return };
    set_static_db(&mut app, ci, -6.0);
    add_node(&mut app, ci, 100.0, -3.0);
    add_node(&mut app, ci, 900.0, -3.0);
    app.paint(&mut c, None);
    let g = geom(&app);
    let cl = clip(&app, ci);
    let body = g.body(&cl);
    let x = body.x + body.w * 0.5;
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    hover(&mut c, &mut app, x, y);
    assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainLine));
    // -6 static + flat -3 line = -9.0 dB at any pointer between the nodes.
    assert_eq!(app.hint.as_ref(), "Clip gain line: -9.0 dB (click to add a node, right-click for options)");
}

// =====================================================================================
// GAIN-SNAP-SPEC.md, S1–S4
// =====================================================================================
//
// Tolerances (stated once, used below):
// - SNAP_TOL_MS = 0.01 ms: the spec's own tolerance for a snapped time.
// - FREE_TOL_MS = 0.05 ms: a free node must sit at the pointer's time. Pointer x is f32 near
//   ~700 px (ulp ~6e-5 px) at ~2.3 ms/px, so f32 noise is ~1e-4 ms; 0.05 ms is still 5x
//   smaller than the 0.4-tick (~1.9 ms) offset the free cases use to tell "free" from
//   "rounded to a whole tick".

const SNAP_TOL_MS: f64 = 0.01;
const FREE_TOL_MS: f64 = 0.05;

use crate::snap::EditorSnap;

/// Grid unit in song ticks and ms per song tick, both through the public API (S2).
fn unit_and_mpt(app: &App) -> (u32, f64) {
    let g = geom(app);
    (pl::grid_snap(&app.state).ticks(g.ppq, g.bar_w), pl::fade_ms_per_tick(&app.state))
}

/// Spec S2 formula: song tick = position + ms/mpt, rounded to the nearest multiple of u
/// (song ticks), back to clip-relative ms, clamped to 0..=clip_ms.
fn expect_snapped(app: &App, ci: usize, free_ms: f64) -> f64 {
    let (u, mpt) = unit_and_mpt(app);
    assert!(u > 0, "test precondition: grid on");
    let cl = clip(app, ci);
    let pos = cl.position as f64;
    let t = pos + free_ms / mpt;
    let snapped_t = (t / u as f64).round() * u as f64;
    ((snapped_t - pos) * mpt).clamp(0.0, pl::clip_ms(&app.state, &cl) as f64)
}

/// Clip-relative ms of a (fractional) song tick.
fn tick_ms(app: &App, ci: usize, song_tick: f64) -> f64 {
    let (_, mpt) = unit_and_mpt(app);
    (song_tick - clip(app, ci).position as f64) * mpt
}

/// Distance (in ticks) of a clip-relative time from the nearest whole song tick.
fn tick_frac_dist(app: &App, ci: usize, ms: f64) -> f64 {
    let (_, mpt) = unit_and_mpt(app);
    let t = clip(app, ci).position as f64 + ms / mpt;
    (t - t.round()).abs()
}

/// Screen x of a clip-relative ms, by inverting the public `x_ms` (linear across the body).
fn ms_x(app: &App, ci: usize, ms: f64) -> f32 {
    let g = geom(app);
    let cl = clip(app, ci);
    let body = g.body(&cl);
    let a = pl::x_ms(&app.state, &g, &cl, body.x) as f64;
    let b = pl::x_ms(&app.state, &g, &cl, body.x + 100.0) as f64;
    (body.x as f64 + (ms - a) * 100.0 / (b - a)) as f32
}

/// Hover node k and press on it (asserting the press really hits the node).
fn press_node(c: &mut Canvas, app: &mut App, ci: usize, k: usize) -> (f32, f32) {
    let (nx, ny) = node_xy(app, ci, k);
    hover(c, app, nx, ny);
    assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainNode(k)), "precondition: pointer on node {k}");
    app.mouse_down();
    app.paint(c, None);
    (nx, ny)
}

fn drag_to(c: &mut Canvas, app: &mut App, x: f32, y: f32) {
    app.mouse_move(x, y);
    app.paint(c, None);
}

fn node_ms(app: &App, ci: usize) -> f64 {
    let ns = nodes(app, ci);
    assert_eq!(ns.len(), 1, "exactly one node expected, got {ns:?}");
    ns[0].0 as f64
}

/// Single node at an off-grid, non-whole-tick time (0.3 u + 0.4 tick past the grid point at 0).
fn one_offgrid_node(c: &mut Canvas, app: &mut App, ci: usize, db: f32) -> f64 {
    let (u, _) = unit_and_mpt(app);
    let pos = clip(app, ci).position as f64;
    let first_grid = (pos / u as f64).ceil() * u as f64;
    let m0 = tick_ms(app, ci, first_grid + 0.3 * u as f64 + 0.4);
    add_node(app, ci, m0 as f32, db);
    app.paint(c, None);
    m0
}

// ---------- S1: value-only readout ----------

#[test]
fn gain_snap_s1_readout_is_value_only_on_hover() {
    for (static_db, node_db, ms) in [(-6.0f32, -6.0f32, 500.0f32), (0.0, 4.5, 20.0), (3.0, -27.0, CLIP_MS - 20.0)] {
        let Some((mut c, mut app, ci)) = setup("s1-hover") else { return };
        set_static_db(&mut app, ci, static_db);
        add_node(&mut app, ci, ms, node_db);
        app.paint(&mut c, None);
        let (nx, ny) = node_xy(&app, ci, 0);
        hover(&mut c, &mut app, nx, ny);
        let (text, r) = pl::gain_readout(&app.state, &geom(&app)).expect("hovered node shows a readout");
        let want = fmt_db(whole(&app, ci, 0));
        assert_eq!(text, want, "S1: readout is exactly the whole gain, nothing else");
        assert!(!text.contains("ms"), "S1: no time in the readout: {text:?}");
        assert!(r.h <= 22.0, "S1: one-line pill must be <= 22 px tall, got {r:?}");
        assert!(r.h >= 11.0, "S1: pill still fits >= 11 px text, got {r:?}");
    }
}

#[test]
fn gain_snap_s1_readout_is_value_only_during_drag() {
    let Some((mut c, mut app, ci)) = setup("s1-drag") else { return };
    set_static_db(&mut app, ci, -6.0);
    add_node(&mut app, ci, 300.0, -3.0);
    app.paint(&mut c, None);
    let (nx, ny) = press_node(&mut c, &mut app, ci, 0);
    for k in 1..=6 {
        drag_to(&mut c, &mut app, nx + 12.0 * k as f32, ny + 6.0 * k as f32);
        let (text, r) = pl::gain_readout(&app.state, &geom(&app)).expect("readout while dragging");
        assert_eq!(text, fmt_db(whole(&app, ci, 0)), "frame {k}: readout is exactly the whole gain");
        assert!(!text.contains("ms"), "frame {k}: no time in the readout: {text:?}");
        assert!(r.h <= 22.0 && r.h >= 11.0, "frame {k}: pill height must be 11..=22 px, got {r:?}");
    }
    app.mouse_up();
}

// ---------- S2: drag snaps to the playlist grid ----------

/// Beat grid (coarse, u = one beat): pointer targets 0.3 u and 0.7 u past a grid point
/// must land on the nearer grid point, and visibly NOT at the pointer time.
#[test]
fn gain_snap_s2_drag_snaps_time_to_beat_grid() {
    let Some((mut c, mut app, ci)) = setup("s2-beat") else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
    let (u, mpt) = unit_and_mpt(&app);
    assert!(u as f64 * mpt >= 50.0, "test precondition: beat grid is coarse ({u} ticks)");
    one_offgrid_node(&mut c, &mut app, ci, -4.0);
    let (_, ny) = press_node(&mut c, &mut app, ci, 0);
    let uf = u as f64;
    for (target, want_tick) in [(0.7 * uf, uf), (1.3 * uf, uf), (0.2 * uf, 0.0), (1.8 * uf, 2.0 * uf)] {
        let free_ms = tick_ms(&app, ci, target + 0.4);
        if free_ms > CLIP_MS as f64 {
            continue;
        }
        let px = ms_x(&app, ci, free_ms);
        drag_to(&mut c, &mut app, px, ny);
        let got = node_ms(&app, ci);
        let want = tick_ms(&app, ci, want_tick);
        assert!((want - expect_snapped(&app, ci, free_ms)).abs() < 1e-9, "fixture sanity");
        assert!(
            (got - want).abs() <= SNAP_TOL_MS,
            "pointer at song tick {:.1}: node must snap to tick {want_tick} ({want:.3} ms), got {got:.3} ms",
            target + 0.4
        );
        assert!((got - free_ms).abs() > 1.0, "snapped time must differ from the free pointer time");
    }
    let held = node_ms(&app, ci);
    app.mouse_up();
    let got = node_ms(&app, ci);
    assert!((got - held).abs() <= SNAP_TOL_MS, "release keeps the snapped time: {held} -> {got}");
    assert!((got - expect_snapped(&app, ci, got)).abs() <= SNAP_TOL_MS, "released node is on the grid: {got}");
    let _ = mpt;
}

/// Property: over generated pointer positions and several grids, every dragged node time is
/// exactly the spec formula (nearest multiple of u in song ticks, clamped to the clip).
#[test]
fn gain_snap_s2_property_snapped_time_matches_formula() {
    let mut seed: u64 = 0xD1B5_4A32_D192_ED03;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    // Main = the default (follows the main snap); Line/Beat explicit.
    for snap in [EditorSnap::Main, EditorSnap::Grid(pl::Snap::Line), EditorSnap::Grid(pl::Snap::Beat)] {
        let Some((mut c, mut app, ci)) = setup("s2-prop") else { return };
        app.state.playlist.snap = snap;
        let (u, _) = unit_and_mpt(&app);
        assert!(u > 0, "{snap:?}: precondition: grid on");
        one_offgrid_node(&mut c, &mut app, ci, -2.0);
        let (_, ny) = press_node(&mut c, &mut app, ci, 0);
        let mut off_grid_targets = 0;
        for i in 0..60 {
            // Pointer anywhere in the clip; the last few hug the right edge to hit the clamp.
            let free_ms = if i >= 55 { CLIP_MS as f64 - 4.0 * rnd() } else { 5.0 + (CLIP_MS as f64 - 10.0) * rnd() };
            let px = ms_x(&app, ci, free_ms);
            drag_to(&mut c, &mut app, px, ny);
            let got = node_ms(&app, ci);
            let want = expect_snapped(&app, ci, free_ms);
            assert!(
                (got - want).abs() <= SNAP_TOL_MS,
                "{snap:?} case {i}: pointer {free_ms:.3} ms -> want {want:.3} ms, got {got:.3} ms"
            );
            if (want - free_ms).abs() > 1.0 {
                off_grid_targets += 1;
            }
        }
        assert!(off_grid_targets >= 20, "{snap:?}: generator must mostly land between grid points");
        app.mouse_up();
    }
}

/// Clamp: the nearest grid point past the clip's end is clamped to clip_ms.
#[test]
fn gain_snap_s2_snap_past_clip_end_clamps_to_clip_length() {
    let Some((mut c, mut app, ci)) = setup("s2-clamp") else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Line);
    let (u, mpt) = unit_and_mpt(&app);
    let end_tick = CLIP_MS as f64 / mpt;
    let next = (end_tick / u as f64).ceil() * u as f64;
    // Precondition: the clip end is off-grid and the next grid point is nearer than the previous.
    let prev = next - u as f64;
    let target = end_tick - 0.3;
    assert!(next > end_tick && (next - target) < (target - prev), "fixture: end {end_tick}, grid {prev}/{next}");
    one_offgrid_node(&mut c, &mut app, ci, -2.0);
    let (_, ny) = press_node(&mut c, &mut app, ci, 0);
    let px = ms_x(&app, ci, target * mpt);
    drag_to(&mut c, &mut app, px, ny);
    let got = node_ms(&app, ci);
    assert!((got - CLIP_MS as f64).abs() <= SNAP_TOL_MS, "snap past the end must clamp to {CLIP_MS} ms, got {got}");
    app.mouse_up();
}

/// Rounding is in SONG ticks: a clip that starts half a unit off the grid gets nodes at
/// clip-relative times that are NOT multiples of u (a clip-relative implementation would be).
#[test]
fn gain_snap_s2_off_grid_clip_rounds_in_song_ticks() {
    let beat_u = {
        let Some((_c, mut app, _)) = setup("s2-offgrid-probe") else { return };
        app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
        unit_and_mpt(&app).0
    };
    assert!(beat_u >= 2 && beat_u % 2 == 0, "fixture: need an even unit, got {beat_u}");
    let half = beat_u / 2;
    let Some((mut c, mut app, ci)) = setup_at("s2-offgrid", half) else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
    let (u, mpt) = unit_and_mpt(&app);
    assert_eq!(u, beat_u);
    let uf = u as f64;
    // Node at song tick u + 0.3u (clip-relative 0.8u): song rounding -> song tick u (rel 0.5u);
    // clip-relative rounding would give rel u.
    let m0 = tick_ms(&app, ci, 1.3 * uf + 0.4) as f32;
    add_node(&mut app, ci, m0, -3.0);
    app.paint(&mut c, None);
    let (_, ny) = press_node(&mut c, &mut app, ci, 0);
    for (target, want_song_tick) in [(1.2 * uf, uf), (1.85 * uf, 2.0 * uf), (0.7 * uf, uf)] {
        let free_ms = tick_ms(&app, ci, target + 0.4);
        let px = ms_x(&app, ci, free_ms);
        drag_to(&mut c, &mut app, px, ny);
        let got = node_ms(&app, ci);
        let want = tick_ms(&app, ci, want_song_tick);
        let clip_rel = ((free_ms / mpt / uf).round() * uf * mpt).clamp(0.0, CLIP_MS as f64);
        assert!((want - clip_rel).abs() > 1.0, "fixture: song vs clip-relative rounding must differ");
        assert!(
            (got - want).abs() <= SNAP_TOL_MS,
            "pointer at song tick {:.1}: must snap to SONG tick {want_song_tick} ({want:.3} ms rel), got {got:.3} ms \
             (clip-relative rounding would give {clip_rel:.3})",
            target + 0.4
        );
    }
    app.mouse_up();
}

/// Grid off (magnet "(none)"), Events and Markers: no grid -> time is free, not even whole ticks.
#[test]
fn gain_snap_s2_no_grid_is_free_not_whole_ticks() {
    for snap in [EditorSnap::Grid(pl::Snap::None), EditorSnap::Events, EditorSnap::Markers] {
        let Some((mut c, mut app, ci)) = setup("s2-free") else { return };
        app.state.playlist.snap = snap;
        assert_eq!(pl::grid_snap(&app.state), pl::Snap::None, "{snap:?}: fixture: no grid");
        add_node(&mut app, ci, 200.0, -3.0);
        app.paint(&mut c, None);
        let (_, ny) = press_node(&mut c, &mut app, ci, 0);
        for tick in [37.4f64, 61.6, 100.4, 150.6] {
            let free_ms = tick_ms(&app, ci, tick);
            let px = ms_x(&app, ci, free_ms);
            drag_to(&mut c, &mut app, px, ny);
            let got = node_ms(&app, ci);
            assert!((got - free_ms).abs() <= FREE_TOL_MS, "{snap:?}: free drag to {free_ms:.3} ms, got {got:.3}");
            assert!(tick_frac_dist(&app, ci, got) > 0.1, "{snap:?}: free time must not be rounded to a whole tick: {got}");
        }
        app.mouse_up();
    }
}

/// dB is never snapped: the same diagonal pointer path gives the same dB with the grid on
/// and off, while the time differs.
#[test]
fn gain_snap_s2_db_is_not_snapped() {
    let mut results = Vec::new();
    for snap in [EditorSnap::Grid(pl::Snap::Beat), EditorSnap::Grid(pl::Snap::None)] {
        let Some((mut c, mut app, ci)) = setup("s2-db") else { return };
        app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
        one_offgrid_node(&mut c, &mut app, ci, -1.0);
        app.state.playlist.snap = snap;
        let (nx, ny) = press_node(&mut c, &mut app, ci, 0);
        for k in 1..=7 {
            drag_to(&mut c, &mut app, nx + 13.7 * k as f32, ny + 3.3 * k as f32);
        }
        app.mouse_up();
        results.push(nodes(&app, ci)[0]);
    }
    let ((ms_on, db_on), (ms_off, db_off)) = (results[0], results[1]);
    assert!((ms_on - ms_off).abs() > 1.0, "S2: with the beat grid on the time must snap, unlike the free run ({ms_on} vs {ms_off})");
    // 1e-4 dB: identical pointer path; only f32 noise allowed.
    assert!((db_on - db_off).abs() < 1e-4, "dB must not depend on snapping: {db_on} vs {db_off}");
    assert!(db_on < -2.0, "dragging down lowered the node: {db_on}");
}

/// A press on the line adds a node at the SNAPPED time with the line's own value there.
#[test]
fn gain_snap_s2_line_press_adds_node_at_snapped_time_on_the_line() {
    let Some((mut c, mut app, ci)) = setup("s2-line") else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
    sloped(&mut c, &mut app, ci); // nodes at 100 ms (-12) and 900 ms (+6)
    let (u, _) = unit_and_mpt(&app);
    let free_ms = tick_ms(&app, ci, u as f64 * 1.3 + 0.4);
    let want = expect_snapped(&app, ci, free_ms);
    assert!(want > 100.0 && want < 900.0 && (want - free_ms).abs() > 1.0, "fixture: snapped {want} between nodes");
    let g = geom(&app);
    let cl = clip(&app, ci);
    let probe: Vec<f32> = (0..=100).map(|i| CLIP_MS * i as f32 / 100.0).collect();
    let line0: Vec<f32> = probe.iter().map(|&t| cl.gain_line.db_at(t)).collect();
    let want_db = cl.gain_line.db_at(want as f32);
    let x = ms_x(&app, ci, free_ms);
    let y = pl::gain_line_y_at(&app.state, &g, &cl, x);
    hover(&mut c, &mut app, x, y);
    assert_eq!(app.state.playlist.hover_handle, Some(pl::Handle::GainLine), "precondition: on the line");
    app.mouse_down();
    app.paint(&mut c, None);
    app.mouse_up();
    app.paint(&mut c, None);
    let ns = nodes(&app, ci);
    assert_eq!(ns.len(), 3, "press on the line adds one node: {ns:?}");
    let added = ns.iter().find(|n| n.0 != 100.0 && n.0 != 900.0).copied().expect("new node");
    assert!(
        (added.0 as f64 - want).abs() <= SNAP_TOL_MS,
        "line press at {free_ms:.3} ms must add at snapped {want:.3} ms, got {:.3}",
        added.0
    );
    // 0.01 dB: the node takes the line's value at the snapped time (only f32 interpolation noise).
    assert!((added.1 - want_db).abs() < 0.01, "added node dB {} must equal the line at {want:.3} ms ({want_db})", added.1);
    let after = clip(&app, ci);
    for (t, d0) in probe.iter().zip(&line0) {
        let d1 = after.gain_line.db_at(*t);
        assert!((d1 - d0).abs() < 0.05, "line reshaped at {t} ms: {d0} -> {d1}");
    }
}

// ---------- S3: free movement whenever wanted ----------

#[test]
fn gain_snap_s3_alt_mid_drag_is_free_then_snaps_again() {
    let Some((mut c, mut app, ci)) = setup("s3-alt") else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
    one_offgrid_node(&mut c, &mut app, ci, -4.0);
    let (u, _) = unit_and_mpt(&app);
    let uf = u as f64;
    let (_, ny) = press_node(&mut c, &mut app, ci, 0);
    // Alt pressed AFTER the press: means "no snap", not delete.
    app.alt = true;
    for target in [0.7 * uf + 0.4, 1.3 * uf + 0.4] {
        let free_ms = tick_ms(&app, ci, target);
        let px = ms_x(&app, ci, free_ms);
        drag_to(&mut c, &mut app, px, ny);
        let got = node_ms(&app, ci); // also asserts Alt mid-drag did not delete the node
        assert!((got - free_ms).abs() <= FREE_TOL_MS, "Alt: free 1:1 drag to {free_ms:.3} ms, got {got:.3}");
        assert!(tick_frac_dist(&app, ci, got) > 0.1, "Alt: not rounded to whole ticks: {got}");
    }
    // Release Alt: the next move snaps again.
    app.alt = false;
    let free_ms = tick_ms(&app, ci, 1.2 * uf + 0.4);
    let px = ms_x(&app, ci, free_ms);
    drag_to(&mut c, &mut app, px, ny);
    let got = node_ms(&app, ci);
    let want = tick_ms(&app, ci, uf);
    assert!((got - want).abs() <= SNAP_TOL_MS, "after releasing Alt the node snaps to {want:.3} ms, got {got:.3}");
    app.mouse_up();
    assert_eq!(nodes(&app, ci).len(), 1, "node kept");
}

#[test]
fn gain_snap_s3_shift_drag_is_free_and_fine() {
    let Some((mut c, mut app, ci)) = setup("s3-shift") else { return };
    app.state.playlist.snap = EditorSnap::Grid(pl::Snap::Beat);
    let m0 = one_offgrid_node(&mut c, &mut app, ci, -4.0);
    let (u, mpt) = unit_and_mpt(&app);
    app.shift = true;
    let (_, ny) = press_node(&mut c, &mut app, ci, 0);
    // Pointer moves 0.8 u to the right; at 1/10 the node moves ~0.08 u, staying off-grid
    // (0.38 u past the grid point at 0); a snap would put it on 0 or u.
    let dms = 0.8 * u as f64 * mpt;
    let steps = 8;
    for k in 1..=steps {
        let x = ms_x(&app, ci, m0 + dms * k as f64 / steps as f64);
        drag_to(&mut c, &mut app, x, ny);
    }
    let got = node_ms(&app, ci);
    app.mouse_up();
    app.shift = false;
    let moved = got - m0;
    assert!(
        moved > 0.05 * dms && moved < 0.2 * dms,
        "Shift: time moves at ~1/10 rate: pointer {dms:.2} ms -> node {moved:.3} ms (want ~{:.2})",
        dms / 10.0
    );
    let on_grid = (got - expect_snapped(&app, ci, got)).abs() <= SNAP_TOL_MS;
    assert!(!on_grid, "Shift: time must not snap, got {got:.3} ms");
    assert!(tick_frac_dist(&app, ci, got) > 0.02, "Shift: not rounded to whole ticks: {got}");
}

// ---------- S4: hint ----------

#[test]
fn gain_snap_s4_node_hint_mentions_alt_and_shift() {
    let Some((mut c, mut app, ci)) = setup("s4-node") else { return };
    three_nodes(&mut c, &mut app, ci);
    let (nx, ny) = node_xy(&app, ci, 2);
    hover(&mut c, &mut app, nx, ny);
    assert_eq!(
        app.hint.as_ref(),
        "Clip gain node: -15.0 dB at 750 ms (drag to move, Alt: no snap, Shift: fine, double-click to type, right-click for options)"
    );
}
