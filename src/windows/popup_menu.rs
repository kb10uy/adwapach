use crate::windows::{subclass_window_procedure, SubclassProxy};

use std::{
    ffi::OsString,
    mem::size_of,
    os::windows::prelude::OsStrExt,
    ptr::{null, NonNull},
};

use anyhow::Result;
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{BOOL, HWND},
        UI::{
            Shell::{RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                CreatePopupMenu, DestroyMenu, InsertMenuItemW, SetForegroundWindow,
                TrackPopupMenuEx, HMENU, MENUITEMINFOW, MIIM_ID, MIIM_STRING, WM_COMMAND,
            },
        },
    },
};

pub struct MenuItem(pub &'static str, pub u32);

/// Represents a Windows' popup menu.
pub struct PopupMenu {
    hwnd: HWND,
    hmenu: HMENU,
    proxy_ptr: NonNull<SubclassProxy>,
}

unsafe impl Send for PopupMenu {}
unsafe impl Sync for PopupMenu {}

impl PopupMenu {
    /// Constructs new menu.
    pub fn new(
        hwnd: HWND,
        items: &[MenuItem],
        on_menu_select: impl Fn(u32) + Send + Sync + 'static,
    ) -> Result<PopupMenu> {
        // Create proxy
        let target_menu_ids: Vec<_> = items.iter().map(|mi| mi.1).collect();
        let proxy = SubclassProxy::new(move |_, msg, wparam, _| {
            if msg != WM_COMMAND {
                return false;
            }

            let menu_id = (wparam.0 & 0xFFFF) as u32;
            if !target_menu_ids.contains(&menu_id) {
                return false;
            }

            on_menu_select(menu_id);
            true
        });
        let proxy_ptr = NonNull::new(Box::into_raw(Box::new(proxy))).expect("Should exist");

        // Create menu
        let hmenu = unsafe { CreatePopupMenu() }?;
        for (i, menu_item) in items.iter().enumerate() {
            let menu_string: OsString = menu_item.0.into();
            let mut menu_text_buffer: Vec<_> = menu_string.encode_wide().collect();
            menu_text_buffer.push(0);

            let mii = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_STRING | MIIM_ID,
                wID: menu_item.1,
                dwTypeData: PWSTR(menu_text_buffer.as_mut_ptr()),
                cch: menu_text_buffer.len() as u32,
                ..Default::default()
            };

            unsafe {
                InsertMenuItemW(hmenu, i as u32, BOOL(1), &mii);
            }
        }

        unsafe {
            SetWindowSubclass(
                hwnd,
                Some(subclass_window_procedure),
                proxy_ptr.as_ptr() as usize,
                0,
            );
        }

        Ok(PopupMenu {
            hwnd,
            hmenu,
            proxy_ptr,
        })
    }

    pub fn track_at(&self, x: i32, y: i32) {
        unsafe {
            SetForegroundWindow(self.hwnd);
            TrackPopupMenuEx(self.hmenu, 0, x, y, self.hwnd, null());
        }
    }
}

impl Drop for PopupMenu {
    fn drop(&mut self) {
        unsafe {
            DestroyMenu(self.hmenu);
            RemoveWindowSubclass(
                self.hwnd,
                Some(subclass_window_procedure),
                self.proxy_ptr.as_ptr() as usize,
            );
            drop(Box::from_raw(self.proxy_ptr.as_ptr()));
        }
    }
}
