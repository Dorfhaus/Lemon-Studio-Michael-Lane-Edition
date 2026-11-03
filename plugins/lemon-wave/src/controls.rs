// SPDX-License-Identifier: GPL-3.0-or-later
//! Lemon Wave controls; appended models preserve existing parameter identities.
use lemon_small_fx::{Card, Control, Mark, Setting, Sound, Spec};
use os_ui::Rect;
pub const A_BANK: usize = 0;
pub const A_POSITION: usize = 1;
pub const A_LEVEL: usize = 2;
pub const A_TUNE: usize = 3;
pub const A_UNISON: usize = 4;
pub const A_DETUNE: usize = 5;
pub const A_PAN: usize = 6;
pub const A_WARP: usize = 7;
pub const B_BANK: usize = 8;
pub const B_POSITION: usize = 9;
pub const B_LEVEL: usize = 10;
pub const B_TUNE: usize = 11;
pub const B_UNISON: usize = 12;
pub const B_DETUNE: usize = 13;
pub const B_PAN: usize = 14;
pub const B_WARP: usize = 15;
pub const CUTOFF: usize = 16;
pub const RESONANCE: usize = 17;
pub const FILTER: usize = 18;
pub const DRIVE: usize = 19;
pub const SUB: usize = 20;
pub const NOISE: usize = 21;
pub const GAIN: usize = 22;
pub const SPREAD: usize = 23;
pub const ATTACK: usize = 24;
pub const DECAY: usize = 25;
pub const SUSTAIN: usize = 26;
pub const RELEASE: usize = 27;
pub const ENV_ATTACK: usize = 28;
pub const ENV_DECAY: usize = 29;
pub const ENV_SUSTAIN: usize = 30;
pub const ENV_RELEASE: usize = 31;
pub const ENV_AMOUNT: usize = 32;
pub const LFO_RATE: usize = 33;
pub const LFO_AMOUNT: usize = 34;
pub const LFO_SHAPE: usize = 35;
pub const LFO_TARGET: usize = 36;
pub const LFO_SYNC: usize = 37;
pub const CHORUS_RATE: usize = 38;
pub const CHORUS_DEPTH: usize = 39;
pub const CHORUS_MIX: usize = 40;
pub const DELAY_TIME: usize = 41;
pub const DELAY_FEEDBACK: usize = 42;
pub const DELAY_MIX: usize = 43;
pub const A_SOURCE: usize = 44;
pub const A_MODE: usize = 45;
pub const A_AMOUNT: usize = 46;
pub const B_SOURCE: usize = 47;
pub const B_MODE: usize = 48;
pub const B_AMOUNT: usize = 49;
pub const A_PHASE: usize = 50;
pub const A_RANDOM: usize = 51;
pub const A_BLEND: usize = 52;
pub const B_PHASE: usize = 53;
pub const B_RANDOM: usize = 54;
pub const B_BLEND: usize = 55;
pub const MOD1_SOURCE: usize = 56;
pub const MOD1_TARGET: usize = 57;
pub const MOD1_AMOUNT: usize = 58;
pub const MOD2_SOURCE: usize = 59;
pub const MOD2_TARGET: usize = 60;
pub const MOD2_AMOUNT: usize = 61;
pub const ENV_ENGINE: usize = 62;
pub const AMP_HOLD: usize = 63;
pub const AMP_ATK_CURVE: usize = 64;
pub const AMP_DEC_CURVE: usize = 65;
pub const AMP_REL_CURVE: usize = 66;
pub const ENV_HOLD: usize = 67;
pub const ENV_ATK_CURVE: usize = 68;
pub const ENV_DEC_CURVE: usize = 69;
pub const ENV_REL_CURVE: usize = 70;
pub const FILTER_MODEL: usize = 71;
pub const MOD3_SOURCE: usize = 72;
pub const MOD3_TARGET: usize = 73;
pub const MOD3_AMOUNT: usize = 74;
pub const MOD4_SOURCE: usize = 75;
pub const MOD4_TARGET: usize = 76;
pub const MOD4_AMOUNT: usize = 77;
pub const MOD5_SOURCE: usize = 78;
pub const MOD5_TARGET: usize = 79;
pub const MOD5_AMOUNT: usize = 80;
pub const MOD6_SOURCE: usize = 81;
pub const MOD6_TARGET: usize = 82;
pub const MOD6_AMOUNT: usize = 83;
pub const MOD7_SOURCE: usize = 84;
pub const MOD7_TARGET: usize = 85;
pub const MOD7_AMOUNT: usize = 86;
pub const MOD8_SOURCE: usize = 87;
pub const MOD8_TARGET: usize = 88;
pub const MOD8_AMOUNT: usize = 89;
pub const MACRO1: usize = 90;
pub const MACRO2: usize = 91;
pub const MACRO3: usize = 92;
pub const MACRO4: usize = 93;
pub const LFO_DRAW: usize = 94;
pub const LFO1_MODE: usize = 95;
pub const LFO1_STYLE: usize = 96;
pub const LFO2_RATE: usize = 97;
pub const LFO2_SYNC: usize = 98;
pub const LFO2_MODE: usize = 99;
pub const LFO2_STYLE: usize = 100;
pub const LFO3_RATE: usize = 101;
pub const LFO3_SYNC: usize = 102;
pub const LFO3_MODE: usize = 103;
pub const LFO3_STYLE: usize = 104;
pub const LFO4_RATE: usize = 105;
pub const LFO4_SYNC: usize = 106;
pub const LFO4_MODE: usize = 107;
pub const LFO4_STYLE: usize = 108;
pub const COUNT: usize = 109;
pub const MOD_SOURCES: [&str; 5] = ["Off", "LFO 1", "Envelope 2", "Velocity", "Key track"];
pub const MOD_TARGETS: [&str; 8] = [
    "Pitch",
    "Filter cutoff",
    "A warp",
    "B warp",
    "A position",
    "B position",
    "Pan",
    "Amplitude",
];
pub const ROUTES: [usize; 8] = [
    MOD1_SOURCE,
    MOD2_SOURCE,
    MOD3_SOURCE,
    MOD4_SOURCE,
    MOD5_SOURCE,
    MOD6_SOURCE,
    MOD7_SOURCE,
    MOD8_SOURCE,
];
pub const ALL_MOD_SOURCES: [&str; 13] = [
    "Off",
    "LFO 1",
    "Envelope 2",
    "Velocity",
    "Key track",
    "LFO 2",
    "LFO 3",
    "LFO 4",
    "Macro 1",
    "Macro 2",
    "Macro 3",
    "Macro 4",
    "Envelope 1",
];
pub const ALL_MOD_TARGETS: [&str; 21] = [
    "Pitch",
    "Filter cutoff",
    "A warp",
    "B warp",
    "A position",
    "B position",
    "Pan",
    "Amplitude",
    "Resonance",
    "Drive",
    "A level",
    "B level",
    "Sub level",
    "Noise level",
    "Stereo width",
    "A tune",
    "B tune",
    "A detune",
    "B detune",
    "A blend",
    "B blend",
];
pub const EXTRA_TARGETS: [usize; 13] = [
    RESONANCE, DRIVE, A_LEVEL, B_LEVEL, SUB, NOISE, SPREAD, A_TUNE, B_TUNE, A_DETUNE, B_DETUNE,
    A_BLEND, B_BLEND,
];
pub const LFO_CONTROLS: [[usize; 4]; 4] = [
    [LFO_RATE, LFO_SYNC, LFO1_MODE, LFO1_STYLE],
    [LFO2_RATE, LFO2_SYNC, LFO2_MODE, LFO2_STYLE],
    [LFO3_RATE, LFO3_SYNC, LFO3_MODE, LFO3_STYLE],
    [LFO4_RATE, LFO4_SYNC, LFO4_MODE, LFO4_STYLE],
];
pub fn mod_target(k: usize) -> Option<usize> {
    match k {
        CUTOFF => Some(1),
        A_AMOUNT => Some(2),
        B_AMOUNT => Some(3),
        A_POSITION => Some(4),
        B_POSITION => Some(5),
        A_PAN | B_PAN => Some(6),
        GAIN => Some(7),
        _ => EXTRA_TARGETS.iter().position(|x| *x == k).map(|i| i + 8),
    }
}
pub const WARP_IDS: [u32; 11] = [0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15];
pub const WARP_NAMES: [&str; 11] = [
    "Off", "Bend +", "Bend -", "Bend +/-", "PWM", "Asym +", "Asym -", "Asym +/-", "Flip", "Mirror",
    "Quantize",
];
pub const SETTINGS: [Setting; COUNT] = [
    Setting {
        id: "a_bank",
        name: "A Bank",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_position",
        name: "A Position",
        range: (0, 1000),
        start: 650,
        magnet: None,
    },
    Setting {
        id: "a_level",
        name: "A Level",
        range: (0, 1000),
        start: 700,
        magnet: None,
    },
    Setting {
        id: "a_tune",
        name: "A Tune",
        range: (-24, 24),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_unison",
        name: "A Unison",
        range: (1, 7),
        start: 1,
        magnet: None,
    },
    Setting {
        id: "a_detune",
        name: "A Detune",
        range: (0, 100),
        start: 12,
        magnet: None,
    },
    Setting {
        id: "a_pan",
        name: "A Pan",
        range: (-100, 100),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_warp",
        name: "A Bend",
        range: (-90, 90),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_bank",
        name: "B Bank",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_position",
        name: "B Position",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "b_level",
        name: "B Level",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_tune",
        name: "B Tune",
        range: (-24, 24),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_unison",
        name: "B Unison",
        range: (1, 7),
        start: 1,
        magnet: None,
    },
    Setting {
        id: "b_detune",
        name: "B Detune",
        range: (0, 100),
        start: 12,
        magnet: None,
    },
    Setting {
        id: "b_pan",
        name: "B Pan",
        range: (-100, 100),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_warp",
        name: "B Bend",
        range: (-90, 90),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "cutoff",
        name: "Cutoff",
        range: (0, 1000),
        start: 1000,
        magnet: None,
    },
    Setting {
        id: "resonance",
        name: "Resonance",
        range: (0, 1000),
        start: 70,
        magnet: None,
    },
    Setting {
        id: "filter",
        name: "Filter",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "drive",
        name: "Drive",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "sub",
        name: "Sub",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "noise",
        name: "Noise",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "gain",
        name: "Output",
        range: (0, 1000),
        start: 650,
        magnet: None,
    },
    Setting {
        id: "spread",
        name: "Spread",
        range: (0, 1000),
        start: 800,
        magnet: None,
    },
    Setting {
        id: "attack",
        name: "Attack",
        range: (0, 1000),
        start: 80,
        magnet: None,
    },
    Setting {
        id: "decay",
        name: "Decay",
        range: (0, 1000),
        start: 460,
        magnet: None,
    },
    Setting {
        id: "sustain",
        name: "Sustain",
        range: (0, 1000),
        start: 800,
        magnet: None,
    },
    Setting {
        id: "release",
        name: "Release",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "env_attack",
        name: "Mod attack",
        range: (0, 1000),
        start: 80,
        magnet: None,
    },
    Setting {
        id: "env_decay",
        name: "Mod decay",
        range: (0, 1000),
        start: 450,
        magnet: None,
    },
    Setting {
        id: "env_sustain",
        name: "Mod sustain",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "env_release",
        name: "Mod release",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "env_amount",
        name: "Env > cutoff",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_rate",
        name: "LFO rate",
        range: (0, 1000),
        start: 450,
        magnet: None,
    },
    Setting {
        id: "lfo_amount",
        name: "LFO amount",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_shape",
        name: "LFO shape",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_target",
        name: "LFO target",
        range: (0, 3),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_sync",
        name: "LFO sync",
        range: (0, 8),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "chorus_rate",
        name: "Chorus rate",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "chorus_depth",
        name: "Chorus depth",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "chorus_mix",
        name: "Chorus mix",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "delay_time",
        name: "Delay time",
        range: (0, 1000),
        start: 400,
        magnet: None,
    },
    Setting {
        id: "delay_feedback",
        name: "Feedback",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "delay_mix",
        name: "Delay mix",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_source",
        name: "A Source",
        range: (0, 1),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_mode",
        name: "A Warp mode",
        range: (0, 10),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_amount",
        name: "A Warp amount",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_source",
        name: "B Source",
        range: (0, 1),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_mode",
        name: "B Warp mode",
        range: (0, 10),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_amount",
        name: "B Warp amount",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_phase",
        name: "A Phase",
        range: (0, 360),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_random",
        name: "A Random",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "a_blend",
        name: "A Blend",
        range: (0, 1000),
        start: 750,
        magnet: None,
    },
    Setting {
        id: "b_phase",
        name: "B Phase",
        range: (0, 360),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_random",
        name: "B Random",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "b_blend",
        name: "B Blend",
        range: (0, 1000),
        start: 750,
        magnet: None,
    },
    Setting {
        id: "mod1_source",
        name: "Mod 1 Source",
        range: (0, 4),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod1_target",
        name: "Mod 1 Target",
        range: (0, 7),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod1_amount",
        name: "Mod 1 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod2_source",
        name: "Mod 2 Source",
        range: (0, 4),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod2_target",
        name: "Mod 2 Target",
        range: (0, 7),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod2_amount",
        name: "Mod 2 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "env_engine",
        name: "Envelope Engine",
        range: (0, 1),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "amp_hold",
        name: "Amp Hold",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "amp_atk_curve",
        name: "Amp Attack Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "amp_dec_curve",
        name: "Amp Decay Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "amp_rel_curve",
        name: "Amp Release Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "env_hold",
        name: "Env 2 Hold",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "env_atk_curve",
        name: "Env 2 Attack Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "env_dec_curve",
        name: "Env 2 Decay Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "env_rel_curve",
        name: "Env 2 Release Curve",
        range: (0, 1000),
        start: 500,
        magnet: None,
    },
    Setting {
        id: "filter_model",
        name: "Filter Model",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod3_source",
        name: "Mod 3 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod3_target",
        name: "Mod 3 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod3_amount",
        name: "Mod 3 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod4_source",
        name: "Mod 4 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod4_target",
        name: "Mod 4 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod4_amount",
        name: "Mod 4 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod5_source",
        name: "Mod 5 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod5_target",
        name: "Mod 5 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod5_amount",
        name: "Mod 5 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod6_source",
        name: "Mod 6 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod6_target",
        name: "Mod 6 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod6_amount",
        name: "Mod 6 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod7_source",
        name: "Mod 7 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod7_target",
        name: "Mod 7 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod7_amount",
        name: "Mod 7 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod8_source",
        name: "Mod 8 Source",
        range: (0, 12),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod8_target",
        name: "Mod 8 Target",
        range: (0, 20),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "mod8_amount",
        name: "Mod 8 Amount",
        range: (-1000, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "macro1",
        name: "Macro 1",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "macro2",
        name: "Macro 2",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "macro3",
        name: "Macro 3",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "macro4",
        name: "Macro 4",
        range: (0, 1000),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo_draw",
        name: "LFO 1 Shape Engine",
        range: (0, 1),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo1_mode",
        name: "LFO 1 Trigger",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo1_style",
        name: "LFO 1 Division",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo2_rate",
        name: "LFO 2 Rate",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "lfo2_sync",
        name: "LFO 2 Sync",
        range: (0, 8),
        start: 5,
        magnet: None,
    },
    Setting {
        id: "lfo2_mode",
        name: "LFO 2 Trigger",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo2_style",
        name: "LFO 2 Division",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo3_rate",
        name: "LFO 3 Rate",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "lfo3_sync",
        name: "LFO 3 Sync",
        range: (0, 8),
        start: 5,
        magnet: None,
    },
    Setting {
        id: "lfo3_mode",
        name: "LFO 3 Trigger",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo3_style",
        name: "LFO 3 Division",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo4_rate",
        name: "LFO 4 Rate",
        range: (0, 1000),
        start: 350,
        magnet: None,
    },
    Setting {
        id: "lfo4_sync",
        name: "LFO 4 Sync",
        range: (0, 8),
        start: 5,
        magnet: None,
    },
    Setting {
        id: "lfo4_mode",
        name: "LFO 4 Trigger",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
    Setting {
        id: "lfo4_style",
        name: "LFO 4 Division",
        range: (0, 2),
        start: 0,
        magnet: None,
    },
];
pub const CONTROLS: [Control; 50] = [
    Control::Knob {
        id: "a_bank",
        cx: 59.0,
        cy: 164.0,
        r: 21.0,
        caption: "Bank",
        bipolar: false,
    },
    Control::Knob {
        id: "a_position",
        cx: 146.0,
        cy: 164.0,
        r: 21.0,
        caption: "Position",
        bipolar: false,
    },
    Control::Knob {
        id: "a_level",
        cx: 233.0,
        cy: 164.0,
        r: 21.0,
        caption: "Level",
        bipolar: false,
    },
    Control::Knob {
        id: "a_tune",
        cx: 320.0,
        cy: 164.0,
        r: 21.0,
        caption: "Tune",
        bipolar: true,
    },
    Control::Knob {
        id: "a_unison",
        cx: 59.0,
        cy: 255.0,
        r: 21.0,
        caption: "Unison",
        bipolar: false,
    },
    Control::Knob {
        id: "a_detune",
        cx: 146.0,
        cy: 255.0,
        r: 21.0,
        caption: "Detune",
        bipolar: false,
    },
    Control::Knob {
        id: "a_pan",
        cx: 233.0,
        cy: 255.0,
        r: 21.0,
        caption: "Pan",
        bipolar: true,
    },
    Control::Knob {
        id: "a_warp",
        cx: 320.0,
        cy: 255.0,
        r: 21.0,
        caption: "Bend",
        bipolar: true,
    },
    Control::Knob {
        id: "b_bank",
        cx: 421.0,
        cy: 164.0,
        r: 21.0,
        caption: "Bank",
        bipolar: false,
    },
    Control::Knob {
        id: "b_position",
        cx: 508.0,
        cy: 164.0,
        r: 21.0,
        caption: "Position",
        bipolar: false,
    },
    Control::Knob {
        id: "b_level",
        cx: 595.0,
        cy: 164.0,
        r: 21.0,
        caption: "Level",
        bipolar: false,
    },
    Control::Knob {
        id: "b_tune",
        cx: 682.0,
        cy: 164.0,
        r: 21.0,
        caption: "Tune",
        bipolar: true,
    },
    Control::Knob {
        id: "b_unison",
        cx: 421.0,
        cy: 255.0,
        r: 21.0,
        caption: "Unison",
        bipolar: false,
    },
    Control::Knob {
        id: "b_detune",
        cx: 508.0,
        cy: 255.0,
        r: 21.0,
        caption: "Detune",
        bipolar: false,
    },
    Control::Knob {
        id: "b_pan",
        cx: 595.0,
        cy: 255.0,
        r: 21.0,
        caption: "Pan",
        bipolar: true,
    },
    Control::Knob {
        id: "b_warp",
        cx: 682.0,
        cy: 255.0,
        r: 21.0,
        caption: "Bend",
        bipolar: true,
    },
    Control::Knob {
        id: "cutoff",
        cx: 783.0,
        cy: 164.0,
        r: 21.0,
        caption: "Cutoff",
        bipolar: false,
    },
    Control::Knob {
        id: "resonance",
        cx: 870.0,
        cy: 164.0,
        r: 21.0,
        caption: "Resonance",
        bipolar: false,
    },
    Control::Knob {
        id: "filter",
        cx: 957.0,
        cy: 164.0,
        r: 21.0,
        caption: "Filter",
        bipolar: false,
    },
    Control::Knob {
        id: "drive",
        cx: 1044.0,
        cy: 164.0,
        r: 21.0,
        caption: "Drive",
        bipolar: false,
    },
    Control::Knob {
        id: "sub",
        cx: 783.0,
        cy: 255.0,
        r: 21.0,
        caption: "Sub",
        bipolar: false,
    },
    Control::Knob {
        id: "noise",
        cx: 870.0,
        cy: 255.0,
        r: 21.0,
        caption: "Noise",
        bipolar: false,
    },
    Control::Knob {
        id: "gain",
        cx: 957.0,
        cy: 255.0,
        r: 21.0,
        caption: "Output",
        bipolar: false,
    },
    Control::Knob {
        id: "spread",
        cx: 1044.0,
        cy: 255.0,
        r: 21.0,
        caption: "Spread",
        bipolar: false,
    },
    Control::Knob {
        id: "attack",
        cx: 59.0,
        cy: 414.0,
        r: 21.0,
        caption: "Attack",
        bipolar: false,
    },
    Control::Knob {
        id: "decay",
        cx: 146.0,
        cy: 414.0,
        r: 21.0,
        caption: "Decay",
        bipolar: false,
    },
    Control::Knob {
        id: "sustain",
        cx: 233.0,
        cy: 414.0,
        r: 21.0,
        caption: "Sustain",
        bipolar: false,
    },
    Control::Knob {
        id: "release",
        cx: 320.0,
        cy: 414.0,
        r: 21.0,
        caption: "Release",
        bipolar: false,
    },
    Control::Knob {
        id: "env_attack",
        cx: 405.0,
        cy: 414.0,
        r: 21.0,
        caption: "attack",
        bipolar: false,
    },
    Control::Knob {
        id: "env_decay",
        cx: 470.0,
        cy: 414.0,
        r: 21.0,
        caption: "decay",
        bipolar: false,
    },
    Control::Knob {
        id: "env_sustain",
        cx: 535.0,
        cy: 414.0,
        r: 21.0,
        caption: "sustain",
        bipolar: false,
    },
    Control::Knob {
        id: "env_release",
        cx: 600.0,
        cy: 414.0,
        r: 21.0,
        caption: "release",
        bipolar: false,
    },
    Control::Knob {
        id: "env_amount",
        cx: 665.0,
        cy: 414.0,
        r: 21.0,
        caption: "Env > cutoff",
        bipolar: true,
    },
    Control::Knob {
        id: "lfo_rate",
        cx: 770.0,
        cy: 414.0,
        r: 21.0,
        caption: "rate",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_amount",
        cx: 835.0,
        cy: 414.0,
        r: 21.0,
        caption: "amount",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_shape",
        cx: 900.0,
        cy: 414.0,
        r: 21.0,
        caption: "shape",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_target",
        cx: 965.0,
        cy: 414.0,
        r: 21.0,
        caption: "target",
        bipolar: false,
    },
    Control::Knob {
        id: "lfo_sync",
        cx: 1030.0,
        cy: 414.0,
        r: 21.0,
        caption: "sync",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_rate",
        cx: 85.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus rate",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_depth",
        cx: 265.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus depth",
        bipolar: false,
    },
    Control::Knob {
        id: "chorus_mix",
        cx: 445.0,
        cy: 590.0,
        r: 21.0,
        caption: "Chorus mix",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_time",
        cx: 625.0,
        cy: 590.0,
        r: 21.0,
        caption: "Delay time",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_feedback",
        cx: 805.0,
        cy: 590.0,
        r: 21.0,
        caption: "Feedback",
        bipolar: false,
    },
    Control::Knob {
        id: "delay_mix",
        cx: 985.0,
        cy: 590.0,
        r: 21.0,
        caption: "Delay mix",
        bipolar: false,
    },
    Control::Choice {
        id: "a_source",
        x: 40.,
        top: 730.,
        names: &["Wavetable / Sample", "Reference Saw"],
    },
    Control::Knob {
        id: "a_mode",
        cx: 220.,
        cy: 744.,
        r: 21.,
        caption: "Warp mode",
        bipolar: false,
    },
    Control::Knob {
        id: "a_amount",
        cx: 314.,
        cy: 744.,
        r: 21.,
        caption: "Amount",
        bipolar: false,
    },
    Control::Choice {
        id: "b_source",
        x: 402.,
        top: 730.,
        names: &["Wavetable / Sample", "Reference Saw"],
    },
    Control::Knob {
        id: "b_mode",
        cx: 582.,
        cy: 744.,
        r: 21.,
        caption: "Warp mode",
        bipolar: false,
    },
    Control::Knob {
        id: "b_amount",
        cx: 676.,
        cy: 744.,
        r: 21.,
        caption: "Amount",
        bipolar: false,
    },
];
const CARDS: [Card; 9] = [
    Card {
        x0: 16.,
        x1: 362.,
        y0: 36.,
        y1: 310.,
        heading: "OSCILLATOR A",
    },
    Card {
        x0: 378.,
        x1: 724.,
        y0: 36.,
        y1: 310.,
        heading: "OSCILLATOR B",
    },
    Card {
        x0: 740.,
        x1: 1084.,
        y0: 36.,
        y1: 310.,
        heading: "FILTER / MIXER",
    },
    Card {
        x0: 16.,
        x1: 362.,
        y0: 350.,
        y1: 480.,
        heading: "AMPLITUDE ENVELOPE",
    },
    Card {
        x0: 378.,
        x1: 724.,
        y0: 350.,
        y1: 480.,
        heading: "FILTER ENVELOPE",
    },
    Card {
        x0: 740.,
        x1: 1084.,
        y0: 350.,
        y1: 480.,
        heading: "LFO / MODULATION",
    },
    Card {
        x0: 16.,
        x1: 1084.,
        y0: 530.,
        y1: 660.,
        heading: "STEREO EFFECTS",
    },
    Card {
        x0: 16.,
        x1: 362.,
        y0: 690.,
        y1: 798.,
        heading: "A SOURCE / WARP",
    },
    Card {
        x0: 378.,
        x1: 724.,
        y0: 690.,
        y1: 798.,
        heading: "B SOURCE / WARP",
    },
];
pub const SPEC: &Spec = &Spec {
    name: "Lemon Wave",
    word: "WAVE",
    about: "Lemon Wave — wavetable synthesis / verified reference oscillator / embedded WAV tables",
    settings: &SETTINGS,
    version: Some(1),
    any_version: false,
    fields: &[],
    tail: &[],
    size: (1180, 820),
    cards: &CARDS,
    controls: &CONTROLS,
    display: None, // Per-instance waveforms are drawn by TableEditor.
    mark: Mark {
        x: 1084.,
        y: 828.,
        anchor: 1.,
        stacked: false,
    },
    hint: Some(Rect::new(16., 814., 780., 26.)),
    labels: &[(910., 76., "POLYPHONIC WAVETABLE SYNTH")],
    text,
    parse,
    sound: unused_sound,
    rings: true,
};
fn unused_sound(_: u32, _: &[i32]) -> Box<dyn Sound> {
    unreachable!("Wave owns its instrument engine")
}
pub fn seconds(v: f32) -> f32 {
    0.001 * 10000.0_f32.powf(v / 1000.)
}
pub fn cutoff(v: f32) -> f32 {
    20. * 1000.0_f32.powf(v / 1000.)
}
pub fn lfo_hz(v: f32) -> f32 {
    0.05 * 400.0_f32.powf(v / 1000.)
}
pub fn text(k: usize, v: i32) -> String {
    if ROUTES[2..].contains(&k) {
        return ALL_MOD_SOURCES[v.clamp(0, 12) as usize].into();
    }
    if ROUTES[2..].iter().any(|s| k == s + 1) {
        return ALL_MOD_TARGETS[v.clamp(0, 20) as usize].into();
    }
    if LFO_CONTROLS.iter().any(|s| k == s[2]) {
        return ["Note loop", "Free run", "One shot"][v.clamp(0, 2) as usize].into();
    }
    if LFO_CONTROLS.iter().any(|s| k == s[3]) {
        return ["Straight", "Dotted", "Triplet"][v.clamp(0, 2) as usize].into();
    }
    if LFO_CONTROLS[1..].iter().any(|s| k == s[1]) {
        return text(LFO_SYNC, v);
    }
    if LFO_CONTROLS[1..].iter().any(|s| k == s[0]) {
        return text(LFO_RATE, v);
    }
    match k {
        LFO_DRAW => ["Classic shape", "Drawn shape"][v.clamp(0, 1) as usize].into(),
        ENV_ENGINE => ["Classic", "Curved AHDSR"][v.clamp(0, 1) as usize].into(),
        FILTER_MODEL => ["Classic", "SVF 12", "SVF 24"][v.clamp(0, 2) as usize].into(),
        AMP_HOLD | ENV_HOLD => format!("{:.0} ms", hold_seconds(v as f32) * 1000.),
        A_PHASE | B_PHASE => format!("{v} deg"),
        MOD1_SOURCE | MOD2_SOURCE => MOD_SOURCES[v.clamp(0, 4) as usize].into(),
        MOD1_TARGET | MOD2_TARGET => MOD_TARGETS[v.clamp(0, 7) as usize].into(),
        A_SOURCE | B_SOURCE => ["Wavetable / Sample", "Reference Saw"][v.clamp(0, 1) as usize].into(),
        A_MODE | B_MODE => WARP_NAMES[v.clamp(0, 10) as usize].into(),
        A_BANK | B_BANK => ["Basic", "Hollow", "Glass", "Vowel"][v.clamp(0, 3) as usize].into(),
        FILTER => ["Bypass", "Low pass", "Band pass", "High pass"][v.clamp(0, 3) as usize].into(),
        LFO_SHAPE => ["Sine", "Triangle", "Ramp", "Square"][v.clamp(0, 3) as usize].into(),
        LFO_TARGET => {
            ["Position / warp", "Cutoff", "Pitch", "Amplitude"][v.clamp(0, 3) as usize].into()
        }
        LFO_SYNC => [
            "Free", "4 bars", "2 bars", "1 bar", "1/2", "1/4", "1/8", "1/16", "1/32",
        ][v.clamp(0, 8) as usize]
            .into(),
        A_TUNE | B_TUNE => format!("{v:+} st"),
        A_UNISON | B_UNISON => format!("{v} voices"),
        A_DETUNE | B_DETUNE => format!("{v} cents"),
        CUTOFF => format!("{:.0} Hz", cutoff(v as f32)),
        ATTACK | DECAY | RELEASE | ENV_ATTACK | ENV_DECAY | ENV_RELEASE => {
            format!("{:.0} ms", seconds(v as f32) * 1000.)
        }
        LFO_RATE => format!("{:.2} Hz", lfo_hz(v as f32)),
        DELAY_TIME => format!("{} ms", 50 + v),
        CHORUS_RATE => format!("{:.2} Hz", 0.05 + v as f32 / 200.),
        _ => format!("{:.1}%", 100. * (v as f32 / SETTINGS[k].range.1 as f32)),
    }
}
fn parse(k: usize, s: &str) -> Option<i32> {
    let s = s.trim();
    if ROUTES[2..].iter().any(|s| k == *s || k == s + 1)
        || LFO_CONTROLS.iter().any(|s| s[1..].contains(&k))
        || k == LFO_DRAW
        || matches!(
            k,
            ENV_ENGINE
                | FILTER_MODEL
                | A_BANK
                | B_BANK
                | FILTER
                | LFO_SHAPE
                | LFO_TARGET
                | LFO_SYNC
                | A_SOURCE
                | B_SOURCE
                | A_MODE
                | B_MODE
                | MOD1_SOURCE
                | MOD2_SOURCE
                | MOD1_TARGET
                | MOD2_TARGET
        )
    {
        return (SETTINGS[k].range.0..=SETTINGS[k].range.1)
            .find(|&v| text(k, v).eq_ignore_ascii_case(s));
    }
    let x = s
        .split_whitespace()
        .next()?
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()?;
    if !x.is_finite() {
        return None;
    }
    let v = match k {
        AMP_HOLD | ENV_HOLD => (x.max(0.) / 32000.).powf(0.2) * 1000.,
        ATTACK | DECAY | RELEASE | ENV_ATTACK | ENV_DECAY | ENV_RELEASE => {
            x.max(1.).log10() / 4. * 1000.
        }
        CUTOFF => (x.max(20.) / 20.).log10() / 3. * 1000.,
        LFO_RATE | LFO2_RATE | LFO3_RATE | LFO4_RATE => {
            (x.max(0.05) / 0.05).ln() / 400.0_f32.ln() * 1000.
        }
        CHORUS_RATE => (x - 0.05) * 200.,
        DELAY_TIME => x - 50.,
        A_TUNE | B_TUNE | A_UNISON | B_UNISON | A_DETUNE | B_DETUNE | A_PHASE | B_PHASE => x,
        _ => x / 100. * SETTINGS[k].range.1 as f32,
    };
    Some((v.round() as i32).clamp(SETTINGS[k].range.0, SETTINGS[k].range.1))
}

pub fn hold_seconds(v: f32) -> f32 {
    32. * (v / 1000.).powi(5)
}
