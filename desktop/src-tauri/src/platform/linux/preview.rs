/// Native preview sink for Wayland / GTK surface.
/// Invariant D06: Native preview on Wayland/Tauri uses waylandsink targeting
/// native surface; never passing raw frames through JS IPC.
#[derive(Default)]
pub struct NativePreviewSink {
    active: bool,
    surface_handle: Option<String>,
}

impl NativePreviewSink {
    pub fn new() -> Self {
        Self {
            active: false,
            surface_handle: None,
        }
    }

    pub fn attach_surface(&mut self, surface_id: &str) {
        self.surface_handle = Some(surface_id.to_string());
    }

    pub fn start(&mut self) -> Result<(), &'static str> {
        self.active = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
