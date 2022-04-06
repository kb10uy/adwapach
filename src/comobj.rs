use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{bail, Result};
use winapi::{
    ctypes::c_void as VOID,
    shared::{
        minwindef::ULONG,
        winerror::{HRESULT, SUCCEEDED},
    },
    um::{
        combaseapi::{CoInitializeEx, CoUninitialize},
        objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE},
    },
};

thread_local! {
    /// Manages initialization count of `ComLibrary`.
    static COM_LIBRARY_COUNT: AtomicUsize = AtomicUsize::new(0);
}

/// Generates typed null.
macro_rules! null {
    ($t: ty) => {
        0 as *mut $t
    };
}

/// The extension trait for `HRESULT` type.
pub trait HresultErrorExt {
    fn err(self) -> Result<()>;
}

impl HresultErrorExt for HRESULT {
    fn err(self) -> Result<()> {
        if SUCCEEDED(self) {
            Ok(())
        } else {
            bail!("HRESULT error value: 0x{:X}", self);
        }
    }
}

impl HresultErrorExt for ULONG {
    fn err(self) -> Result<()> {
        if self == 0 {
            Ok(())
        } else {
            bail!("ULONG error value: 0x{:X}", self);
        }
    }
}

/// Represents a reference for thread-local COM library.
pub struct ComLibrary;

impl ComLibrary {
    /// Creates a reference for COM library.
    pub fn new() -> Result<ComLibrary> {
        COM_LIBRARY_COUNT.with(|c| {
            let new_count = c.fetch_add(1, Ordering::SeqCst);
            if new_count == 1 {
                ComLibrary::initialize_real()
            } else {
                Ok(())
            }
        })?;
        Ok(ComLibrary)
    }

    /// Calls CoInitializeEx.
    fn initialize_real() -> Result<()> {
        unsafe {
            CoInitializeEx(
                null!(VOID),
                COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
            )
            .err()?;
        }
        Ok(())
    }

    /// Calls CoUninitialize.
    fn uninitialize_real() {
        unsafe {
            CoUninitialize();
        }
    }
}

impl Drop for ComLibrary {
    fn drop(&mut self) {
        COM_LIBRARY_COUNT.with(|c| {
            let new_count = c.fetch_sub(1, Ordering::SeqCst);
            if new_count == 0 {
                ComLibrary::uninitialize_real();
            }
        });
    }
}
