use cef::rc::{Rc, RcImpl};
use cef::{
    Browser, CefString, Frame, ImplRequestContextHandler, Request, RequestContext,
    ResourceRequestHandler, WrapRequestContextHandler, sys,
};
use std::cell::Cell;

/// ## Reference
///
/// - [`CefRequestContextHandler Class Reference`](https://cef-builds.spotifycdn.com/docs/106.1/classCefRequestContextHandler.html)
pub struct RequestContextHandlerBuilder {
    object: *mut RcImpl<sys::cef_request_context_handler_t, Self>,
    ready: Option<std::rc::Rc<Cell<bool>>>,
}

impl RequestContextHandlerBuilder {
    pub fn build() -> cef::RequestContextHandler {
        cef::RequestContextHandler::new(Self {
            object: core::ptr::null_mut(),
            ready: None,
        })
    }

    /// Builds a handler that sets `ready` to `true` once CEF reports the context initialized.
    ///
    /// CEF calls `on_request_context_initialized` on the browser-process UI thread.
    pub fn build_with_ready(ready: std::rc::Rc<Cell<bool>>) -> cef::RequestContextHandler {
        cef::RequestContextHandler::new(Self {
            object: core::ptr::null_mut(),
            ready: Some(ready),
        })
    }
}

impl WrapRequestContextHandler for RequestContextHandlerBuilder {
    fn wrap_rc(&mut self, object: *mut RcImpl<sys::_cef_request_context_handler_t, Self>) {
        self.object = object;
    }
}

impl Rc for RequestContextHandlerBuilder {
    fn as_base(&self) -> &sys::cef_base_ref_counted_t {
        unsafe {
            let base = &*self.object;
            core::mem::transmute(&base.cef_object)
        }
    }
}

impl Clone for RequestContextHandlerBuilder {
    fn clone(&self) -> Self {
        let object = unsafe {
            let rc_impl = &mut *self.object;
            rc_impl.interface.add_ref();
            rc_impl
        };
        Self {
            object,
            ready: self.ready.clone(),
        }
    }
}

impl ImplRequestContextHandler for RequestContextHandlerBuilder {
    fn on_request_context_initialized(&self, _request_context: Option<&mut RequestContext>) {
        if let Some(ready) = &self.ready {
            ready.set(true);
        }
    }

    /// With an allow-list set (`local_only`), every request first passes its filter.
    fn resource_request_handler(
        &self,
        _browser: Option<&mut Browser>,
        _frame: Option<&mut Frame>,
        _request: Option<&mut Request>,
        _is_navigation: ::std::os::raw::c_int,
        _is_download: ::std::os::raw::c_int,
        _request_initiator: Option<&CefString>,
        _disable_default_handling: Option<&mut ::std::os::raw::c_int>,
    ) -> Option<ResourceRequestHandler> {
        crate::browser_process::local_only::resource_request_handler()
    }

    #[inline]
    fn get_raw(&self) -> *mut sys::_cef_request_context_handler_t {
        self.object.cast()
    }
}
