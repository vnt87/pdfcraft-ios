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
}

impl IosApp {
    pub fn new(app: PdfCraftApp, root: Option<PathBuf>) -> Self {
        let sync = root.clone().map(crate::documents::DocumentSync::new);
        Self { app, root, last_safe_area: None, sync }
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
        self.app.logic(ctx, frame);
        if let Some(sync) = &mut self.sync {
            sync.poll(|message| self.app.notify(message));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw_input);
        crate::safe_area::apply(raw_input);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.app.save(storage);
    }
}
