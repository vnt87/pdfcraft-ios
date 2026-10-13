//! PdfCraft patch: the iPadOS backend's plumbing, with no UIKit in it.
//!
//! rfd has no iOS backend. Every dialog becomes a [`Request`] handed to the one [`Handler`] the app
//! installs with [`set_handler`] at start-up; the handler shows UIKit's document picker (or answers
//! at once) and sends the chosen paths back through the [`Reply`]. Dropping a `Reply` without
//! sending counts as Cancel, so a picker that failed to show cannot hang its caller.

use crate::backend::DialogFutureType;
use crate::oneshot;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::task::{Context, Poll, Wake, Waker};

/// What the dialog asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One existing file.
    OpenFile,
    /// One or more existing files.
    OpenFiles,
    /// One folder.
    Folder,
    /// One or more folders.
    Folders,
    /// A place to write one file.
    Save,
}

/// A file type filter: a name and the extensions it lists (without dots).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// One dialog, as the app's [`Handler`] sees it.
#[derive(Clone, Debug)]
pub struct Request {
    pub kind: Kind,
    pub title: Option<String>,
    /// The suggested name of the file to save.
    pub file_name: Option<String>,
    pub directory: Option<PathBuf>,
    pub filters: Vec<Filter>,
}

/// Where the handler sends the answer: the chosen paths, or `None` for Cancel.
pub struct Reply(oneshot::Sender<Option<Vec<PathBuf>>>);

impl Reply {
    pub fn send(self, paths: Option<Vec<PathBuf>>) {
        // The caller may have gone away; nobody is left to tell.
        let _ = self.0.send(paths);
    }
}

/// Shows the dialog on whatever thread it needs and answers through the `Reply`. It is called on
/// the thread that created the dialog, so it must hop to the main thread itself for UIKit.
pub type Handler = Box<dyn Fn(Request, Reply) + Send + Sync + 'static>;

static HANDLER: OnceLock<Handler> = OnceLock::new();

/// Install the app's handler. Only the first call takes effect; later ones get the handler back.
pub fn set_handler(handler: Handler) -> Result<(), Handler> {
    HANDLER.set(handler)
}

/// Hand `request` to the handler. With no handler installed the dialog is cancelled.
pub(crate) fn ask(request: Request) -> DialogFutureType<Option<Vec<PathBuf>>> {
    let (tx, rx) = oneshot::channel();
    match HANDLER.get() {
        Some(handler) => handler(request, Reply(tx)),
        None => drop(tx),
    }
    Box::pin(async move { rx.await.ok().flatten() })
}

struct Unpark(std::thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Wait for `future` on the calling thread (the synchronous dialogs; never call them on the main thread).
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: Kind) -> Request {
        Request { kind, title: None, file_name: None, directory: None, filters: Vec::new() }
    }

    #[test]
    fn a_dropped_reply_is_a_cancel_and_a_sent_one_arrives() {
        let (tx, rx) = oneshot::channel::<Option<Vec<PathBuf>>>();
        drop(Reply(tx));
        assert_eq!(block_on(async move { rx.await.ok().flatten() }), None);
        let (tx, rx) = oneshot::channel::<Option<Vec<PathBuf>>>();
        Reply(tx).send(Some(vec!["/a.pdf".into()]));
        assert_eq!(block_on(async move { rx.await.ok().flatten() }), Some(vec![PathBuf::from("/a.pdf")]));
    }

    #[test]
    fn the_installed_handler_answers_every_request_and_a_second_install_is_refused() {
        // The handler is process-wide: this is the only test that installs one.
        assert!(HANDLER.get().is_none() || cfg!(not(test)));
        let installed = set_handler(Box::new(|request, reply| match request.kind {
            Kind::Save => reply.send(Some(vec![PathBuf::from(request.file_name.unwrap_or_default())])),
            Kind::OpenFile => reply.send(None),
            // Left unanswered: the reply is dropped, which is a cancel.
            _ => {}
        }));
        assert!(installed.is_ok());
        assert!(set_handler(Box::new(|_, _| {})).is_err());
        let mut save = request(Kind::Save);
        save.file_name = Some("out.pdf".into());
        assert_eq!(block_on(ask(save)), Some(vec![PathBuf::from("out.pdf")]));
        assert_eq!(block_on(ask(request(Kind::OpenFile))), None);
        assert_eq!(block_on(ask(request(Kind::Folders))), None);
    }

    #[test]
    fn block_on_waits_for_an_answer_sent_from_another_thread() {
        let (tx, rx) = oneshot::channel::<Option<Vec<PathBuf>>>();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            Reply(tx).send(Some(vec!["/late.pdf".into()]));
        });
        assert_eq!(block_on(async move { rx.await.ok().flatten() }), Some(vec![PathBuf::from("/late.pdf")]));
        assert!(sender.join().is_ok());
    }
}
