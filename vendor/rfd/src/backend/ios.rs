//! PdfCraft patch: the dialog traits on iPadOS, over [`super::ios_bridge`].

use super::ios_bridge::{ask, block_on, Filter, Kind, Request};
use super::{
    AsyncFilePickerDialogImpl, AsyncFileSaveDialogImpl, AsyncFolderPickerDialogImpl, AsyncMessageDialogImpl,
    DialogFutureType, FilePickerDialogImpl, FileSaveDialogImpl, FolderPickerDialogImpl, MessageDialogImpl,
};
use crate::file_dialog::FileDialog;
use crate::message_dialog::{MessageDialog, MessageDialogResult};
use crate::FileHandle;
use std::path::PathBuf;

fn request(dialog: FileDialog, kind: Kind) -> Request {
    Request {
        kind,
        title: dialog.title,
        file_name: dialog.file_name,
        directory: dialog.starting_directory,
        filters: dialog.filters.into_iter().map(|f| Filter { name: f.name, extensions: f.extensions }).collect(),
    }
}

fn one(paths: Option<Vec<PathBuf>>) -> Option<PathBuf> {
    paths?.into_iter().next()
}

fn handles(paths: Option<Vec<PathBuf>>) -> Option<Vec<FileHandle>> {
    let paths = paths?;
    (!paths.is_empty()).then(|| paths.into_iter().map(FileHandle::wrap).collect())
}

impl FilePickerDialogImpl for FileDialog {
    fn pick_file(self) -> Option<PathBuf> {
        one(block_on(ask(request(self, Kind::OpenFile))))
    }
    fn pick_files(self) -> Option<Vec<PathBuf>> {
        block_on(ask(request(self, Kind::OpenFiles))).filter(|paths| !paths.is_empty())
    }
}

impl FileSaveDialogImpl for FileDialog {
    fn save_file(self) -> Option<PathBuf> {
        one(block_on(ask(request(self, Kind::Save))))
    }
}

impl FolderPickerDialogImpl for FileDialog {
    fn pick_folder(self) -> Option<PathBuf> {
        one(block_on(ask(request(self, Kind::Folder))))
    }
    fn pick_folders(self) -> Option<Vec<PathBuf>> {
        block_on(ask(request(self, Kind::Folders))).filter(|paths| !paths.is_empty())
    }
}

impl AsyncFilePickerDialogImpl for FileDialog {
    fn pick_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        let answer = ask(request(self, Kind::OpenFile));
        Box::pin(async move { one(answer.await).map(FileHandle::wrap) })
    }
    fn pick_files_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        let answer = ask(request(self, Kind::OpenFiles));
        Box::pin(async move { handles(answer.await) })
    }
}

impl AsyncFileSaveDialogImpl for FileDialog {
    fn save_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        let answer = ask(request(self, Kind::Save));
        Box::pin(async move { one(answer.await).map(FileHandle::wrap) })
    }
}

impl AsyncFolderPickerDialogImpl for FileDialog {
    fn pick_folder_async(self) -> DialogFutureType<Option<FileHandle>> {
        let answer = ask(request(self, Kind::Folder));
        Box::pin(async move { one(answer.await).map(FileHandle::wrap) })
    }
    fn pick_folders_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        let answer = ask(request(self, Kind::Folders));
        Box::pin(async move { handles(answer.await) })
    }
}

// Message boxes are not part of this port: nothing answers them, so they cancel (PdfCraft draws its own).
impl MessageDialogImpl for MessageDialog {
    fn show(self) -> MessageDialogResult {
        MessageDialogResult::Cancel
    }
}

impl AsyncMessageDialogImpl for MessageDialog {
    fn show_async(self) -> DialogFutureType<MessageDialogResult> {
        Box::pin(async { MessageDialogResult::Cancel })
    }
}
