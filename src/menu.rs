use windows_sys::w;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateMenu, DrawMenuBar, GetMenuItemCount, RemoveMenu, SetMenu, SetMenuItemInfoW,
    MENUITEMINFOW, MFS_CHECKED, MFS_GRAYED, MFS_UNCHECKED, MF_BYCOMMAND, MF_CHECKED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, MIIM_STATE,
};

use std::{collections, mem, ops, ptr};

pub const IDM_COUNTER: isize = 1;
pub const IDM_COUNTER_NEW: isize = 2;
pub const IDM_COUNTER_REMOVE: isize = 3;
pub const IDM_COUNTER_REMOVE_ALL: isize = 4;
pub const IDM_COUNTER_REMOVE_SEPARATOR: isize = 5;

pub const IDM_LOG: isize = 6;
pub const IDM_LOG_START: isize = 7;
pub const IDM_LOG_STOP: isize = 8;

pub const IDM_REMOVE_RANGE: ops::Range<isize> = 100..200;

pub struct Menu {
    pub hmenu: isize,
    sub_menus: collections::HashMap<isize, Option<isize>>,
    hwnd: isize,
}

impl Menu {
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new(hwnd: HWND) -> Self {
        let mut menu = Menu {
            hmenu: CreateMenu(),
            sub_menus: collections::HashMap::new(),
            hwnd,
        };
        SetMenu(hwnd, menu.hmenu);

        menu.add_menu(None, IDM_COUNTER, w!("&Counter"));
        menu.add_item(Some(IDM_COUNTER), IDM_COUNTER_NEW, w!("&New"), None, false);
        menu.add_menu(Some(IDM_COUNTER), IDM_COUNTER_REMOVE, w!("&Remove"));
        menu.add_item(
            Some(IDM_COUNTER_REMOVE),
            IDM_COUNTER_REMOVE_ALL,
            w!("&Remove All"),
            None,
            false,
        );
        menu.add_separator(Some(IDM_COUNTER_REMOVE), IDM_COUNTER_REMOVE_SEPARATOR);

        menu.add_menu(None, IDM_LOG, w!("&Log"));
        menu.add_item(Some(IDM_LOG), IDM_LOG_START, w!("&Start"), None, false);
        menu.add_item(Some(IDM_LOG), IDM_LOG_STOP, w!("&Stop"), None, true);

        menu
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn add_menu(&mut self, parent_id: Option<isize>, menu_id: isize, name: *const u16) {
        let new_menu = CreateMenu();
        match parent_id {
            Some(parent_id) => {
                AppendMenuW(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    MF_STRING | MF_POPUP,
                    new_menu as usize,
                    name,
                );
            }
            None => {
                AppendMenuW(self.hmenu, MF_STRING | MF_POPUP, new_menu as usize, name);
            }
        };

        self.sub_menus.insert(menu_id, Some(new_menu));
        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn add_item(
        &mut self,
        parent_id: Option<isize>,
        item_id: isize,
        name: *const u16,
        check: Option<bool>,
        grayed: bool,
    ) {
        let mut uflags = MF_STRING;

        if let Some(check_status) = check {
            if check_status {
                uflags |= MF_CHECKED;
            } else {
                uflags |= MF_UNCHECKED;
            }
        };

        if grayed {
            uflags |= MF_GRAYED;
        };

        match parent_id {
            Some(parent_id) => {
                AppendMenuW(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    uflags,
                    item_id as usize,
                    name,
                );
            }
            None => {
                AppendMenuW(self.hmenu, uflags, item_id as usize, name);
            }
        };

        self.sub_menus.insert(item_id, None);
        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn remove_item(&mut self, menu_id: Option<isize>, item_id: isize) {
        match menu_id {
            Some(parent_id) => {
                RemoveMenu(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    item_id as u32,
                    MF_BYCOMMAND,
                );
            }
            None => {
                RemoveMenu(self.hmenu, item_id as u32, MF_BYCOMMAND);
            }
        };

        self.sub_menus.remove(&item_id);
        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn add_separator(&mut self, id: Option<isize>, separator_id: isize) {
        match id {
            Some(parent_id) => {
                AppendMenuW(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    MF_SEPARATOR,
                    separator_id as usize,
                    ptr::null(),
                );
            }
            None => {
                AppendMenuW(self.hmenu, MF_SEPARATOR, separator_id as usize, ptr::null());
            }
        };

        self.sub_menus.insert(separator_id, None);
        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn set_item_state_by_id(
        &mut self,
        parent_id: Option<isize>,
        item_id: isize,
        check: Option<bool>,
        grayed: bool,
    ) {
        let mut item_info: MENUITEMINFOW = mem::zeroed();
        item_info.cbSize = mem::size_of::<MENUITEMINFOW>() as _;
        item_info.fMask = MIIM_STATE;

        if let Some(check_status) = check {
            if check_status {
                item_info.fState |= MFS_CHECKED;
            } else {
                item_info.fState |= MFS_UNCHECKED;
            }
        };

        if grayed {
            item_info.fState |= MFS_GRAYED;
        };

        match parent_id {
            Some(parent_id) => {
                SetMenuItemInfoW(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    item_id as u32,
                    false.into(),
                    &item_info,
                );
            }
            None => {
                SetMenuItemInfoW(self.hmenu, item_id as u32, false.into(), &item_info);
            }
        };

        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn set_item_state_by_pos(
        &mut self,
        parent_id: Option<isize>,
        item_pos: isize,
        check: Option<bool>,
        grayed: bool,
    ) {
        let mut item_info: MENUITEMINFOW = mem::zeroed();
        item_info.cbSize = mem::size_of::<MENUITEMINFOW>() as _;
        item_info.fMask = MIIM_STATE;

        if let Some(check_status) = check {
            if check_status {
                item_info.fState |= MFS_CHECKED;
            } else {
                item_info.fState |= MFS_UNCHECKED;
            }
        };

        if grayed {
            item_info.fState |= MFS_GRAYED;
        };

        match parent_id {
            Some(parent_id) => {
                SetMenuItemInfoW(
                    self.sub_menus.get(&parent_id).unwrap().unwrap(),
                    item_pos as u32,
                    true.into(),
                    &item_info,
                );
            }
            None => {
                SetMenuItemInfoW(self.hmenu, item_pos as u32, true.into(), &item_info);
            }
        };

        DrawMenuBar(self.hwnd);
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_item_count(&self, menu_id: Option<isize>) -> i32 {
        match menu_id {
            Some(menu_id) => GetMenuItemCount(self.sub_menus.get(&menu_id).unwrap().unwrap()),
            None => GetMenuItemCount(self.hmenu),
        }
    }
}
