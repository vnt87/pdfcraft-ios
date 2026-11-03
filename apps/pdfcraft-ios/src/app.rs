//! The app eframe runs on iPadOS: PdfCraft's UI plus what the host owns (safe area, taking saved
//! documents to where the user keeps them, saving the settings when the app leaves the foreground).
use std::path::PathBuf;

use eframe::egui;
use pdfcraft_ui_egui::PdfCraftApp;

pub struct IosApp {
    app: PdfCraftApp,
    /// The app's support folder (none: nowhere to keep documents and settings).
    root: Option<PathBuf>,
    last_safe_area: Option<String>,
    /// Takes the documents the app saves to where the user keeps them.
    sync: Option<crate::documents::DocumentSync>,
    /// Hardware-keyboard state: the keys winit never delivers (Enter, arrows, ⌘ shortcuts).
    keys: crate::hw_keys::HwKeys,
    /// Events produced outside the input hook (releasing keys on focus loss), sent with the next frame.
    pending: Vec<egui::Event>,
}

impl IosApp {
    pub fn new(app: PdfCraftApp, root: Option<PathBuf>) -> Self {
        let sync = root.clone().map(crate::documents::DocumentSync::new);
        Self { app, root, last_safe_area: None, sync, keys: Default::default(), pending: Vec::new() }
    }

    /// Recent files that are managed copies in an earlier install of the app container.
    pub fn remap_recent_files(&mut self) {
        let Some(root) = &self.root else { return };
        let mut paths: Vec<String> = self.app.recent.iter().map(|recent| recent.path.clone()).collect();
        if crate::storage::remap_recent_files(&mut paths, root) {
            for (recent, path) in self.app.recent.iter_mut().zip(paths) {
                recent.path = path;
            }
        }
    }
}

impl eframe::App for IosApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        crate::safe_area::report(ctx, &mut self.last_safe_area);
        if !ctx.input(|i| i.focused) {
            // Key-ups may never arrive once the app is in the background.
            self.pending.extend(self.keys.release_all());
        }
        self.app.logic(ctx, frame);
        if let Some(sync) = &mut self.sync {
            sync.poll(|message| self.app.notify(message));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // The keys winit's UIKit view never forwards: read the HID transitions and inject them.
        let now = raw_input.time.unwrap_or_default();
        for (code, pressed) in crate::gc_keyboard::drain() {
            raw_input.events.extend(self.keys.handle(code, pressed, now));
        }
        raw_input.events.append(&mut self.pending);
        raw_input.events.extend(self.keys.tick(now));
        if let Some(wait) = self.keys.next_wake(now) {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait.max(0.001)));
        }
        self.app.raw_input_hook(ctx, raw_input);
        crate::safe_area::apply(raw_input);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.app.save(storage);
    }
}
