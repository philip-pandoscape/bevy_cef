#[cfg(target_os = "macos")]
pub mod accelerated_paint;
mod app;
mod browser_process_handler;
mod browsers;
mod client_handler;
mod command_line_config;
mod context_menu_handler;
mod cookie_flush;
pub mod display_handler;
pub mod drag_handler;
mod extensions;
mod initial_zoom;
mod life_span_handler;
pub mod load_handler;
mod local_only;
mod localhost;
mod message_pump;
mod profiles;
mod renderer_handler;
mod request_context_handler;
#[cfg(target_os = "windows")]
mod windows_paint;

pub use app::*;
pub use browser_process_handler::*;
pub use browsers::*;
pub use client_handler::*;
pub use command_line_config::*;
pub use context_menu_handler::*;
pub use cookie_flush::*;
pub use display_handler::*;
pub use drag_handler::*;
pub use extensions::*;
pub use initial_zoom::*;
pub use life_span_handler::*;
pub use load_handler::*;
pub use local_only::{allow_only_schemes, scheme_allowed};
pub use localhost::*;
pub use message_pump::*;
pub use renderer_handler::*;
pub use request_context_handler::*;
#[cfg(target_os = "windows")]
pub use windows_paint::{
    accelerated_paint_enabled, embedder_begin_frames, set_accelerated_paint_handler,
    set_embedder_begin_frames, set_windowless_frame_rate, windowless_frame_rate,
};
