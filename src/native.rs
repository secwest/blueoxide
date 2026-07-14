use crate::{Error, Result};
use std::ffi::{CStr, CString, c_char, c_void};
use std::mem::{size_of, transmute_copy};
use std::ptr::NonNull;

pub struct DynamicLibrary {
    handle: NonNull<c_void>,
    name: String,
}

// SAFETY: Windows module handles and POSIX dlopen handles may be used for
// symbol lookup from multiple threads. DynamicLibrary never mutates the module;
// Drop closes it only after ownership and all Arc references have ended.
unsafe impl Send for DynamicLibrary {}
// SAFETY: See the Send justification above. Symbol lookup does not expose
// mutable Rust state through DynamicLibrary.
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    pub fn open_candidates(candidates: &[&str]) -> Result<Self> {
        if candidates.is_empty() {
            return Err(Error::InvalidConfiguration(
                "native library candidate list is empty".to_owned(),
            ));
        }
        let mut failures = Vec::new();
        for candidate in candidates {
            match platform::open(candidate) {
                Ok(handle) => {
                    return Ok(Self {
                        handle,
                        name: (*candidate).to_owned(),
                    });
                }
                Err(message) => failures.push(format!("{candidate}: {message}")),
            }
        }
        Err(Error::NativeLibrary {
            library: candidates.join(", "),
            message: failures.join("; "),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Loads a native symbol and copies its pointer bits into a function pointer.
    ///
    /// # Safety
    ///
    /// `T` must be the exact function-pointer type exported for `name`. The
    /// returned function pointer may not outlive this `DynamicLibrary`.
    pub unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T> {
        if size_of::<T>() != size_of::<*mut c_void>() {
            return Err(Error::InvalidConfiguration(format!(
                "native symbol type for {name} has size {}, expected {}",
                size_of::<T>(),
                size_of::<*mut c_void>()
            )));
        }
        let name_c = CString::new(name).map_err(|_| {
            Error::InvalidConfiguration("native symbol name contains a NUL octet".to_owned())
        })?;
        // SAFETY: self.handle is live and name_c is a NUL-terminated symbol.
        let pointer = unsafe { platform::symbol(self.handle, &name_c) }.map_err(|message| {
            Error::NativeLibrary {
                library: self.name.clone(),
                message: format!("missing symbol {name}: {message}"),
            }
        })?;
        // SAFETY: The caller guarantees T is the exact exported function type,
        // and the size equality is checked above.
        Ok(unsafe { transmute_copy::<*mut c_void, T>(&pointer.as_ptr()) })
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        // SAFETY: The handle was returned by platform::open and is owned here.
        unsafe { platform::close(self.handle) };
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::ffi::OsStrExt;

    type HModule = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> HModule;
        fn GetProcAddress(module: HModule, name: *const c_char) -> *mut c_void;
        fn FreeLibrary(module: HModule) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn open(name: &str) -> std::result::Result<NonNull<c_void>, String> {
        let wide: Vec<u16> = std::ffi::OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: `wide` is NUL-terminated and valid for the duration of call.
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        NonNull::new(handle).ok_or_else(|| {
            // SAFETY: GetLastError has no preconditions.
            format!("Windows error {}", unsafe { GetLastError() })
        })
    }

    pub unsafe fn symbol(
        handle: NonNull<c_void>,
        name: &CStr,
    ) -> std::result::Result<NonNull<c_void>, String> {
        // SAFETY: handle is a loaded module and name is NUL-terminated.
        let pointer = unsafe { GetProcAddress(handle.as_ptr(), name.as_ptr()) };
        NonNull::new(pointer).ok_or_else(|| {
            // SAFETY: GetLastError has no preconditions.
            format!("Windows error {}", unsafe { GetLastError() })
        })
    }

    pub unsafe fn close(handle: NonNull<c_void>) {
        // SAFETY: handle is an owned module handle.
        let _ = unsafe { FreeLibrary(handle.as_ptr()) };
    }
}

#[cfg(unix)]
mod platform {
    use super::*;

    const RTLD_NOW: i32 = 2;
    #[cfg(target_os = "macos")]
    const RTLD_LOCAL: i32 = 4;
    #[cfg(not(target_os = "macos"))]
    const RTLD_LOCAL: i32 = 0;

    #[cfg_attr(not(target_os = "macos"), link(name = "dl"))]
    unsafe extern "C" {
        fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(handle: *mut c_void) -> i32;
        fn dlerror() -> *const c_char;
    }

    fn last_error() -> String {
        // SAFETY: dlerror returns either NULL or a process-owned C string.
        let pointer = unsafe { dlerror() };
        if pointer.is_null() {
            "unknown dynamic loader error".to_owned()
        } else {
            // SAFETY: Non-NULL dlerror result is NUL-terminated.
            unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn open(name: &str) -> std::result::Result<NonNull<c_void>, String> {
        let name = CString::new(name).map_err(|_| "library name contains NUL".to_owned())?;
        // SAFETY: name is NUL-terminated and flags are valid.
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW | RTLD_LOCAL) };
        NonNull::new(handle).ok_or_else(last_error)
    }

    pub unsafe fn symbol(
        handle: NonNull<c_void>,
        name: &CStr,
    ) -> std::result::Result<NonNull<c_void>, String> {
        // Clear a stale loader error before dlsym.
        // SAFETY: dlerror has no preconditions.
        let _ = unsafe { dlerror() };
        // SAFETY: handle is loaded and name is NUL-terminated.
        let pointer = unsafe { dlsym(handle.as_ptr(), name.as_ptr()) };
        NonNull::new(pointer).ok_or_else(last_error)
    }

    pub unsafe fn close(handle: NonNull<c_void>) {
        // SAFETY: handle is an owned dlopen handle.
        let _ = unsafe { dlclose(handle.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn loads_and_calls_windows_system_symbol() {
        let library = DynamicLibrary::open_candidates(&["kernel32.dll"]).unwrap();
        type GetCurrentProcessId = unsafe extern "system" fn() -> u32;
        // SAFETY: This is the documented kernel32 function signature.
        let function: GetCurrentProcessId =
            unsafe { library.symbol("GetCurrentProcessId") }.unwrap();
        // SAFETY: The function has no arguments or additional preconditions.
        assert_ne!(unsafe { function() }, 0);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn loads_and_calls_unix_system_symbol() {
        let library = DynamicLibrary::open_candidates(&["libc.so.6", "libc.so"]).unwrap();
        type Strlen = unsafe extern "C" fn(*const c_char) -> usize;
        // SAFETY: This is the standard C strlen signature.
        let function: Strlen = unsafe { library.symbol("strlen") }.unwrap();
        let value = c"blueoxide";
        // SAFETY: value is a valid NUL-terminated string.
        assert_eq!(unsafe { function(value.as_ptr()) }, 9);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn loads_and_calls_macos_system_symbol() {
        let library = DynamicLibrary::open_candidates(&["/usr/lib/libSystem.B.dylib"]).unwrap();
        type Strlen = unsafe extern "C" fn(*const c_char) -> usize;
        // SAFETY: This is the standard C strlen signature.
        let function: Strlen = unsafe { library.symbol("strlen") }.unwrap();
        let value = c"blueoxide";
        // SAFETY: value is a valid NUL-terminated string.
        assert_eq!(unsafe { function(value.as_ptr()) }, 9);
    }
}
