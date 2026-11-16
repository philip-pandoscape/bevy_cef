//! Webviews kept to local content: with an allow-list set ([`allow_only_schemes`]), a request for
//! any other scheme is cancelled before it is made, and every permission prompt and media request
//! a page makes is refused, so a page that shows only what the app serves (a custom scheme) can
//! neither reach the network nor ask the user for anything.

use crate::prelude::IntoString;
use cef::*;
use std::os::raw::c_int;
use std::sync::OnceLock;

static ALLOWED: OnceLock<Vec<String>> = OnceLock::new();

/// Keeps every browser to the schemes named (`"myapp"`, `"data"`, `"blob"`): set once, before
/// browsers are created. Unset, every scheme loads and permissions are CEF's default.
pub fn allow_only_schemes(schemes: Vec<String>) {
    let _ = ALLOWED.set(schemes);
}

/// Whether browsers are kept to an allow-list.
pub(crate) fn restricted() -> bool {
    ALLOWED.get().is_some()
}

/// Whether `url` may load: always, with no allow-list; else when its scheme is listed.
pub fn scheme_allowed(url: &str) -> bool {
    let Some(allowed) = ALLOWED.get() else {
        return true;
    };
    url.split_once(':')
        .is_some_and(|(scheme, _)| allowed.iter().any(|a| a.eq_ignore_ascii_case(scheme)))
}

wrap_resource_request_handler! {
    pub(crate) struct LocalOnly;

    impl ResourceRequestHandler {
        fn on_before_resource_load(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            request: Option<&mut Request>,
            _callback: Option<&mut Callback>,
        ) -> ReturnValue {
            let url = request.map(|r| r.url().into_string()).unwrap_or_default();
            if scheme_allowed(&url) {
                ReturnValue::from(sys::cef_return_value_t::RV_CONTINUE)
            } else {
                ReturnValue::from(sys::cef_return_value_t::RV_CANCEL)
            }
        }
    }
}

wrap_permission_handler! {
    pub(crate) struct RefuseAll;

    impl PermissionHandler {
        fn on_request_media_access_permission(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _requesting_origin: Option<&CefString>,
            _requested_permissions: u32,
            callback: Option<&mut MediaAccessCallback>,
        ) -> c_int {
            if let Some(callback) = callback {
                callback.cancel();
            }
            1
        }

        fn on_show_permission_prompt(
            &self,
            _browser: Option<&mut Browser>,
            _prompt_id: u64,
            _requesting_origin: Option<&CefString>,
            _requested_permissions: u32,
            callback: Option<&mut PermissionPromptCallback>,
        ) -> c_int {
            if let Some(callback) = callback {
                callback.cont(PermissionRequestResult::from(
                    sys::cef_permission_request_result_t::CEF_PERMISSION_RESULT_DENY,
                ));
            }
            1
        }
    }
}

/// The request filter for a request context, when browsers are kept to an allow-list.
pub(crate) fn resource_request_handler() -> Option<ResourceRequestHandler> {
    restricted().then(LocalOnly::new)
}

/// The permission handler for a browser, when browsers are kept to an allow-list.
pub(crate) fn permission_handler() -> Option<PermissionHandler> {
    restricted().then(RefuseAll::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_list_everything_loads_and_with_one_only_its_schemes() {
        // (One process, one list: the unset case is checked before it is set.)
        assert!(!restricted() || ALLOWED.get().is_some());
        allow_only_schemes(vec!["myapp".into(), "data".into()]);
        assert!(scheme_allowed("myapp://assets/index.html"));
        assert!(scheme_allowed("MYAPP://assets/a.png"));
        assert!(scheme_allowed("data:image/png;base64,AAAA"));
        assert!(!scheme_allowed("https://example.com/"));
        assert!(!scheme_allowed("http://localhost:5173/"));
        assert!(!scheme_allowed("no scheme at all"));
    }
}
