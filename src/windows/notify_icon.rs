use crate::windows::{subclass_window_procedure, SubclassProxy};

use std::{ffi::OsString, mem::size_of, os::windows::ffi::OsStrExt, ptr::NonNull};

use anyhow::{Context, Result};
use uuid::Uuid;
use windows::{
    core::GUID,
    Win32::{
        Foundation::{HINSTANCE, HWND},
        UI::{
            Shell::{
                RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW, NIF_GUID, NIF_ICON,
                NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFYICONDATAW,
                NOTIFYICONDATAW_0, NOTIFYICON_VERSION_4,
            },
            WindowsAndMessaging::{CreateIcon, DestroyIcon, WM_APP},
        },
    },
};

pub struct NotifyIcon {
    hwnd: HWND,
    uuid: Uuid,
    proxy_ptr: NonNull<SubclassProxy>,
}

unsafe impl Send for NotifyIcon {}
unsafe impl Sync for NotifyIcon {}

impl NotifyIcon {
    pub fn new<F: Fn(u16, (i16, i16)) + Send + Sync + 'static>(
        hwnd: HWND,
        callback_id: u32,
        tooltip: &str,
        icon_image: &[u8],
        on_event: F,
    ) -> Result<NotifyIcon> {
        // NotifyIconProxy as forgotten pointer
        let notify_mid = WM_APP + callback_id;
        let proxy = SubclassProxy::new(move |_, msg, wparam, lparam| {
            if msg != notify_mid {
                return false;
            }

            on_event(
                (lparam.0 & 0xFFFF) as u16,
                ((wparam.0 & 0xFFFF) as i16, (wparam.0 >> 16) as i16),
            );
            true
        });
        let proxy_ptr = NonNull::new(Box::into_raw(Box::new(proxy))).expect("Should exist");

        // HICON
        let (xor_image, and_image, w, h) = create_icon_buffer(icon_image)?;
        let hicon = unsafe {
            CreateIcon(
                HINSTANCE(0),
                w as i32,
                h as i32,
                1,
                32,
                and_image.as_ptr(),
                xor_image.as_ptr(),
            )?
        };

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
            hIcon: hicon,
            uCallbackMessage: notify_mid,
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
                Some(subclass_window_procedure),
                proxy_ptr.as_ptr() as usize,
                0,
            );
            DestroyIcon(hicon);
        }

        Ok(NotifyIcon {
            hwnd,
            uuid,
            proxy_ptr,
        })
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
                Some(subclass_window_procedure),
                self.proxy_ptr.as_ptr() as usize,
            );
            drop(Box::from_raw(self.proxy_ptr.as_ptr()));
        }
    }
}

/// Creates Windows icon bitmap from image file bytes.
pub fn create_icon_buffer(image_memory: &[u8]) -> Result<(Vec<u8>, Vec<u8>, u32, u32)> {
    let icon_image = image::load_from_memory(image_memory)?;
    let mut bgra_image =
        Vec::with_capacity((icon_image.width() * icon_image.height() * 4) as usize);
    let mut mask_image = Vec::with_capacity((icon_image.width() * icon_image.height()) as usize);
    for pixel in icon_image.as_rgba8().context("Not 8bit RGBA")?.pixels() {
        bgra_image.extend_from_slice(&[pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]]);
        mask_image.push(pixel.0[3].wrapping_sub(u8::MAX));
    }
    Ok((
        bgra_image,
        mask_image,
        icon_image.width(),
        icon_image.height(),
    ))
}
