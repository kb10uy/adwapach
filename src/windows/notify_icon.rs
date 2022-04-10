use std::{ffi::OsString, mem::size_of, os::windows::ffi::OsStrExt, ptr::NonNull};

use anyhow::Result;
use uuid::Uuid;
use windows::{
    core::GUID,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{
                DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW,
                NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION,
                NOTIFYICONDATAW, NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
            },
            WindowsAndMessaging::WM_APP,
        },
    },
};

pub struct NotifyIcon {
    hwnd: HWND,
    uuid: Uuid,
    callback_id: u32,
    proxy_ptr: NonNull<NotifyIconProxy>,
}

struct NotifyIconProxy {
    callback_id: u32,
    on_event: Box<dyn Fn(u16, (i16, i16))>,
}

impl NotifyIcon {
    pub fn new<F: Fn(u16, (i16, i16)) + 'static>(
        hwnd: HWND,
        callback_id: u32,
        tooltip: &str,
        on_event: F,
    ) -> Result<NotifyIcon> {
        // NotifyIconProxy as forgotten pointer
        let proxy_ptr = NonNull::new(Box::into_raw(Box::new(NotifyIconProxy {
            callback_id,
            on_event: Box::new(on_event),
        })))
        .expect("Should exist");

        // Icon GUID
        let uuid = Uuid::new_v4();
        let guid = {
            let uuid_fields = uuid.as_fields();
            GUID {
                data1: uuid_fields.0,
                data2: uuid_fields.1,
                data3: uuid_fields.2,
                data4: uuid_fields.3.to_owned(),
            }
        };

        // Tooltip supports currently ASCII only
        let tooltip_string: OsString = if tooltip.is_ascii() {
            tooltip.into()
        } else {
            "".into()
        };
        let mut tooltip_wides: Vec<_> = tooltip_string.encode_wide().collect();
        tooltip_wides.resize(127, 0);
        tooltip_wides.push(0);

        // Construct NOTIFYICONDATA
        let mut nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            uFlags: NIF_GUID | NIF_TIP | NIF_ICON | NIF_MESSAGE,
            guidItem: guid,
            hWnd: hwnd,
            uCallbackMessage: WM_APP + callback_id,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            ..Default::default()
        };
        nid.szTip.copy_from_slice(&tooltip_wides);

        unsafe {
            Shell_NotifyIconW(NIM_ADD, &nid);
            Shell_NotifyIconW(NIM_SETVERSION, &nid);
            SetWindowSubclass(
                hwnd,
                Some(NotifyIcon::subclass_window_procedure),
                callback_id as usize,
                proxy_ptr.as_ptr() as usize,
            );
        }

        Ok(NotifyIcon {
            hwnd,
            uuid,
            callback_id,
            proxy_ptr,
        })
    }

    /// Processes subclass message.
    unsafe extern "system" fn subclass_window_procedure(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        let proxy = &mut *(data as *mut NotifyIconProxy);
        if msg != WM_APP + proxy.callback_id {
            return DefSubclassProc(hwnd, msg, wparam, lparam);
        }

        let message = (lparam.0 & 0xFFFF) as u16;
        let x = (wparam.0 & 0xFFFF) as i16;
        let y = (wparam.0 >> 16) as i16;
        (proxy.on_event)(message, (x, y));

        LRESULT(1)
    }
}

impl Drop for NotifyIcon {
    fn drop(&mut self) {
        let guid = {
            let uuid_fields = self.uuid.as_fields();
            GUID {
                data1: uuid_fields.0,
                data2: uuid_fields.1,
                data3: uuid_fields.2,
                data4: uuid_fields.3.to_owned(),
            }
        };

        let nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            uFlags: NIF_GUID,
            guidItem: guid,
            hWnd: self.hwnd,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            ..Default::default()
        };

        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &nid);
            RemoveWindowSubclass(
                self.hwnd,
                Some(NotifyIcon::subclass_window_procedure),
                self.callback_id as usize,
            );
        }

        drop(unsafe { Box::from_raw(self.proxy_ptr.as_ptr()) });
    }
}
