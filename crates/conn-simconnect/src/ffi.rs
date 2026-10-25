//! Minimal SimConnect C API, resolved from SimConnect.dll at runtime.
//!
//! Loading the DLL at runtime (instead of linking SimConnect.lib) means the
//! MSFS SDK is not needed to build FlightDeck and no SDK files live in the
//! repository. Only the handful of calls FlightDeck uses are declared.

use std::ffi::{c_char, c_void, CString};
use std::path::{Path, PathBuf};

use libloading::Library;

pub type Handle = *mut c_void;
pub type HResult = i32;

pub const S_OK: HResult = 0;

// SIMCONNECT_RECV_ID
pub const RECV_ID_EXCEPTION: u32 = 1;
pub const RECV_ID_OPEN: u32 = 2;
pub const RECV_ID_QUIT: u32 = 3;
pub const RECV_ID_EVENT: u32 = 4;
pub const RECV_ID_SIMOBJECT_DATA: u32 = 8;

// SIMCONNECT_DATATYPE
pub const DATATYPE_FLOAT64: u32 = 4;
pub const DATATYPE_STRING256: u32 = 9;

// SIMCONNECT_PERIOD
pub const PERIOD_ONCE: u32 = 1;
pub const PERIOD_SIM_FRAME: u32 = 3;

pub const OBJECT_ID_USER: u32 = 0;
pub const UNUSED: u32 = u32::MAX;

type OpenFn = unsafe extern "system" fn(*mut Handle, *const c_char, *mut c_void, u32, Handle, u32) -> HResult;
type CloseFn = unsafe extern "system" fn(Handle) -> HResult;
type AddToDataDefinitionFn =
    unsafe extern "system" fn(Handle, u32, *const c_char, *const c_char, u32, f32, u32) -> HResult;
type RequestDataOnSimObjectFn =
    unsafe extern "system" fn(Handle, u32, u32, u32, u32, u32, u32, u32, u32) -> HResult;
type SubscribeToSystemEventFn = unsafe extern "system" fn(Handle, u32, *const c_char) -> HResult;
type GetNextDispatchFn = unsafe extern "system" fn(Handle, *mut *mut u8, *mut u32) -> HResult;

/// The loaded DLL and its function pointers.
pub struct Api {
    _lib: Library,
    open: OpenFn,
    close: CloseFn,
    add_to_data_definition: AddToDataDefinitionFn,
    request_data_on_sim_object: RequestDataOnSimObjectFn,
    subscribe_to_system_event: SubscribeToSystemEventFn,
    get_next_dispatch: GetNextDispatchFn,
}

// The function pointers are plain C functions; SimConnect handles are used
// from one thread at a time (the connector's polling thread).
unsafe impl Send for Api {}

/// Places SimConnect.dll is looked for, in order.
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("FLIGHTDECK_SIMCONNECT_DLL") {
        v.push(PathBuf::from(p));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        v.push(dir.join("SimConnect.dll"));
    }
    for sdk in [r"C:\MSFS 2024 SDK", r"C:\MSFS SDK"] {
        v.push(PathBuf::from(sdk).join(r"SimConnect SDK\lib\SimConnect.dll"));
    }
    if let Ok(sdk) = std::env::var("MSFS_SDK") {
        v.push(PathBuf::from(sdk).join(r"SimConnect SDK\lib\SimConnect.dll"));
    }
    // Last resort: let Windows search its normal DLL paths.
    v.push(PathBuf::from("SimConnect.dll"));
    v
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {
        // SAFETY: the symbol types match the SimConnect.h declarations above.
        unsafe { *$lib.get($name).map_err(|e| format!("{}: {e}", String::from_utf8_lossy($name)))? }
    };
}

impl Api {
    pub fn load() -> Result<Api, String> {
        let mut errors = Vec::new();
        for path in candidate_paths() {
            // SAFETY: loading SimConnect.dll runs its DllMain, which is trusted.
            match unsafe { Library::new(&path) } {
                Ok(lib) => return Self::from_library(lib),
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        Err(format!("SimConnect.dll not found. Tried:\n{}", errors.join("\n")))
    }

    fn from_library(lib: Library) -> Result<Api, String> {
        let open = sym!(lib, b"SimConnect_Open\0");
        let close = sym!(lib, b"SimConnect_Close\0");
        let add_to_data_definition = sym!(lib, b"SimConnect_AddToDataDefinition\0");
        let request_data_on_sim_object = sym!(lib, b"SimConnect_RequestDataOnSimObject\0");
        let subscribe_to_system_event = sym!(lib, b"SimConnect_SubscribeToSystemEvent\0");
        let get_next_dispatch = sym!(lib, b"SimConnect_GetNextDispatch\0");
        Ok(Api {
            _lib: lib,
            open,
            close,
            add_to_data_definition,
            request_data_on_sim_object,
            subscribe_to_system_event,
            get_next_dispatch,
        })
    }

    pub fn open(&self, name: &str) -> Result<Handle, HResult> {
        let name = CString::new(name).unwrap();
        let mut h: Handle = std::ptr::null_mut();
        // SAFETY: valid out-pointer and NUL-terminated name; no window/event.
        let hr = unsafe { (self.open)(&mut h, name.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut(), 0) };
        if hr == S_OK && !h.is_null() {
            Ok(h)
        } else {
            Err(hr)
        }
    }

    pub fn close(&self, h: Handle) {
        // SAFETY: h came from a successful open and is closed once.
        unsafe { (self.close)(h) };
    }

    pub fn add_to_data_definition(&self, h: Handle, def: u32, name: &str, units: Option<&str>, datatype: u32) -> HResult {
        let name = CString::new(name).unwrap();
        let units = units.map(|u| CString::new(u).unwrap());
        let units_ptr = units.as_ref().map_or(std::ptr::null(), |u| u.as_ptr());
        // SAFETY: pointers are valid NUL-terminated strings for the call's duration.
        unsafe { (self.add_to_data_definition)(h, def, name.as_ptr(), units_ptr, datatype, 0.0, UNUSED) }
    }

    pub fn request_data(&self, h: Handle, req: u32, def: u32, period: u32) -> HResult {
        // SAFETY: plain integer arguments.
        unsafe { (self.request_data_on_sim_object)(h, req, def, OBJECT_ID_USER, period, 0, 0, 0, 0) }
    }

    pub fn subscribe(&self, h: Handle, event: u32, name: &str) -> HResult {
        let name = CString::new(name).unwrap();
        // SAFETY: valid NUL-terminated event name.
        unsafe { (self.subscribe_to_system_event)(h, event, name.as_ptr()) }
    }

    /// Copy the next queued message out of SimConnect's buffer, if any.
    pub fn next_dispatch(&self, h: Handle) -> Option<Vec<u8>> {
        let mut ptr: *mut u8 = std::ptr::null_mut();
        let mut len: u32 = 0;
        // SAFETY: valid out-pointers; the returned buffer is only read before
        // the next call into SimConnect, and copied immediately.
        let hr = unsafe { (self.get_next_dispatch)(h, &mut ptr, &mut len) };
        if hr != S_OK || ptr.is_null() || len < 12 {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(ptr, len as usize) }.to_vec())
    }
}
