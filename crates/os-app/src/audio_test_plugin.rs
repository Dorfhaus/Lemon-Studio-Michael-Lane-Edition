//! Controlled native-independent fixture for restore contention, never shipped.
use os_host::{Descriptor, Library, Plugin, SharedPlugin};
use std::{
    ffi::c_void,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
pub struct Synthetic {
    desc: Descriptor,
    delay: u64,
    active: bool,
    level: u8,
}
pub struct Factory {
    desc: Descriptor,
    delay: u64,
}
pub fn library(delay: u64) -> Arc<dyn Library> {
    Arc::new(Factory {
        delay,
        desc: Descriptor {
            format: os_host::Format::Vst3,
            id: "test-slow-restore".into(),
            name: "Synthetic restore".into(),
            vendor: "Test fixture".into(),
            version: format!("delay-{delay}ms"),
            features: vec![],
            kind: os_host::Kind::Instrument,
            path: "synthetic".into(),
        },
    })
}
impl Library for Factory {
    fn path(&self) -> &Path {
        &self.desc.path
    }
    fn plugins(&self) -> &[Descriptor] {
        std::slice::from_ref(&self.desc)
    }
    fn create(self: Arc<Self>, _: &str) -> Result<SharedPlugin, String> {
        Ok(Arc::new(Mutex::new(Synthetic {
            desc: self.desc.clone(),
            delay: self.delay,
            active: false,
            level: 1,
        })))
    }
}
impl os_engine::Instrument for Synthetic {
    fn note_on(&mut self, _: u32, _: u8, _: f32) {
        self.active = true;
    }
    fn note_off(&mut self, _: u32, _: u8) {
        self.active = false;
    }
    fn process(&mut self, _: u32, l: &mut [f32], r: &mut [f32]) {
        l.fill(if self.active { self.level as f32 * 0.1 } else { 0. });
        r.copy_from_slice(l);
    }
    fn parallel_processing(&self) -> bool {
        true
    }
}
impl os_engine::HostedEffect for Synthetic {
    fn process_fx(&mut self, _: u32, _: &mut [f32], _: &mut [f32]) {}
}
impl Plugin for Synthetic {
    fn desc(&self) -> &Descriptor {
        &self.desc
    }
    fn activate(&mut self, _: u32) -> bool {
        true
    }
    fn idle(&mut self) {}
    fn has_editor(&self) -> bool {
        false
    }
    fn editor_open(&self) -> bool {
        false
    }
    fn open_editor(&mut self, _: *mut c_void, _: f32) -> Result<(u32, u32), String> {
        Err("headless fixture".into())
    }
    fn editor_size(&self) -> Option<(u32, u32)> {
        None
    }
    fn close_editor(&mut self) {}
    fn save_state(&self) -> Option<Vec<u8>> {
        Some(vec![self.level])
    }
    fn load_state(&mut self, data: &[u8]) -> bool {
        if data != [1] && data != [2] {
            return false;
        }
        std::thread::sleep(Duration::from_millis(self.delay));
        self.level = data[0];
        self.active = false;
        true
    }
}
