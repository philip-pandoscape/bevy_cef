//! Windows GPU OSR: CEF's shared textures handed to the embedder.
//!
//! With a handler set ([`set_accelerated_paint_handler`]) before a browser is created, the browser
//! is created with `shared_texture_enabled`, and CEF delivers each frame of its main view as a
//! shared D3D11 texture through `on_accelerated_paint` instead of a CPU buffer through `on_paint`.
//! The handler gets the webview's entity and CEF's `AcceleratedPaintInfo`. The texture's handle is
//! valid only during the call: a handler imports it there (for example with the `cef` crate's
//! `osr_texture_import`, into the device it draws with) and copies it before CEF takes the buffer
//! back into its pool.
//!
//! Without a handler, Windows keeps the CPU path. CEF's UI thread is Bevy's main thread under
//! `external_message_pump`, so the handler lives in a thread local and runs on that thread.

use bevy::prelude::Entity;
use cef::AcceleratedPaintInfo;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Handler = Rc<dyn Fn(Entity, &AcceleratedPaintInfo)>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
    static FRAME_RATE: Cell<i32> = const { Cell::new(60) };
    static EMBEDDER_BEGIN_FRAMES: Cell<bool> = const { Cell::new(false) };
}

/// Lets the embedder drive compositing on Windows: browsers created from now on are created with
/// `external_begin_frame_enabled`, and paint only when asked
/// ([`crate::prelude::Browsers::begin_frame`]), so a page's frames can be made in step with the
/// app's own instead of on CEF's timer. bevy_cef sends none itself on Windows: an embedder that
/// sets this sends them. Off unless set (see [`crate::prelude::EXTERNAL_BEGIN_FRAME`] for why).
pub fn set_embedder_begin_frames(on: bool) {
    EMBEDDER_BEGIN_FRAMES.with(|b| b.set(on));
}

/// Whether browsers created now wait for the embedder's begin frames.
pub fn embedder_begin_frames() -> bool {
    EMBEDDER_BEGIN_FRAMES.with(Cell::get)
}

/// Asks CEF for shared GPU textures on Windows, and hands each frame of a webview's main view to
/// `handler`. Set it on the main thread before the browsers it is for are created (in a plugin's
/// `build`, say); browsers created before keep the CPU path.
pub fn set_accelerated_paint_handler(handler: impl Fn(Entity, &AcceleratedPaintInfo) + 'static) {
    HANDLER.with(|h| *h.borrow_mut() = Some(Rc::new(handler)));
}

/// Whether browsers created now ask for shared textures.
pub fn accelerated_paint_enabled() -> bool {
    HANDLER.with(|h| h.borrow().is_some())
}

/// Hands a frame to the handler, if there is one.
pub(crate) fn deliver(webview: Entity, info: &AcceleratedPaintInfo) {
    let handler = HANDLER.with(|h| h.borrow().clone());
    if let Some(handler) = handler {
        handler(webview, info);
    }
}

/// How many times a second browsers created from now on paint (CEF's `windowless_frame_rate`, 60
/// unless set; CEF takes 1 to 240 on Windows). [`crate::prelude::Browsers::set_frame_rate`]
/// changes a browser that exists.
pub fn set_windowless_frame_rate(rate: i32) {
    FRAME_RATE.with(|r| r.set(rate.clamp(1, 240)));
}

/// The rate browsers are created with.
pub fn windowless_frame_rate() -> i32 {
    FRAME_RATE.with(Cell::get)
}
