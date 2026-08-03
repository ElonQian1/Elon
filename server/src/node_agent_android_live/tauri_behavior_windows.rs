use anyhow::{bail, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeBehaviorSnapshot {
    pub(super) descendant_process_count: usize,
    pub(super) windows: Vec<ObservedWindow>,
    pub(super) dialogs: Vec<ObservedWindow>,
    pub(super) menus: Vec<ObservedMenuItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ObservedWindow {
    #[serde(skip)]
    handle: isize,
    pub(super) title: String,
    pub(super) class_name: String,
    pub(super) process_id: u32,
    pub(super) owner_process_id: Option<u32>,
    pub(super) bounds: WindowBounds,
    pub(super) dialog_candidate: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WindowBounds {
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ObservedMenuItem {
    pub(super) label: String,
    pub(super) path: String,
    pub(super) command_id: Option<u32>,
    pub(super) enabled: bool,
    pub(super) checked: bool,
    pub(super) separator: bool,
    pub(super) children: Vec<ObservedMenuItem>,
}

#[cfg(windows)]
pub(super) fn capture_native_behavior(root_process_id: u32) -> Result<NativeBehaviorSnapshot> {
    use std::collections::{HashMap, HashSet};
    use std::mem::size_of;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM, RECT},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetClassNameW, GetMenu, GetWindow, GetWindowRect, GetWindowTextLengthW,
            GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, GW_OWNER,
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        bail!("TAURI_PROCESS_SNAPSHOT_FAILED：无法枚举 Tauri 行为证据进程树");
    }
    let mut parents = HashMap::new();
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
    while ok != 0 {
        parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
        ok = unsafe { Process32NextW(snapshot, &mut entry) };
    }
    unsafe { CloseHandle(snapshot) };
    let allowed = descendants(root_process_id, &parents);
    let mut windows = Vec::<ObservedWindow>::new();
    struct Context<'a> {
        allowed: &'a HashSet<u32>,
        windows: &'a mut Vec<ObservedWindow>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut Context<'_>);
        if IsWindowVisible(hwnd) == 0 || context.windows.len() >= 32 {
            return 1;
        }
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if !context.allowed.contains(&process_id) {
            return 1;
        }
        let title_length = GetWindowTextLengthW(hwnd).clamp(0, 1_000);
        let mut title_buffer = vec![0u16; title_length as usize + 1];
        let title_count =
            GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), title_buffer.len() as i32);
        let title = String::from_utf16_lossy(&title_buffer[..title_count.max(0) as usize])
            .trim()
            .to_string();
        let mut class_buffer = vec![0u16; 256];
        let class_count = GetClassNameW(hwnd, class_buffer.as_mut_ptr(), class_buffer.len() as i32);
        let class_name =
            String::from_utf16_lossy(&class_buffer[..class_count.max(0) as usize]).to_string();
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return 1;
        }
        let owner = GetWindow(hwnd, GW_OWNER);
        let mut owner_process_id = 0u32;
        if !owner.is_null() {
            GetWindowThreadProcessId(owner, &mut owner_process_id);
        }
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        if width < 80 || height < 40 || (title.is_empty() && class_name.is_empty()) {
            return 1;
        }
        context.windows.push(ObservedWindow {
            handle: hwnd as isize,
            title,
            class_name: class_name.clone(),
            process_id,
            owner_process_id: (owner_process_id != 0).then_some(owner_process_id),
            bounds: WindowBounds {
                left: rect.left,
                top: rect.top,
                width,
                height,
            },
            dialog_candidate: class_name == "#32770" || !owner.is_null(),
        });
        1
    }
    let mut context = Context {
        allowed: &allowed,
        windows: &mut windows,
    };
    unsafe { EnumWindows(Some(visit), &mut context as *mut Context<'_> as LPARAM) };
    windows.sort_by_key(|window| {
        std::cmp::Reverse(u64::from(window.bounds.width) * u64::from(window.bounds.height))
    });
    let dialogs = windows
        .iter()
        .filter(|window| window.dialog_candidate)
        .cloned()
        .collect::<Vec<_>>();
    let mut menu_count = 0usize;
    let menus = windows
        .iter()
        .find_map(|window| {
            let menu = unsafe { GetMenu(window.handle as HWND) };
            (!menu.is_null()).then(|| read_menu(menu, "", 0, &mut menu_count))
        })
        .transpose()?
        .unwrap_or_default();
    Ok(NativeBehaviorSnapshot {
        descendant_process_count: allowed.len(),
        windows,
        dialogs,
        menus,
    })
}

#[cfg(windows)]
fn read_menu(
    menu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
    parent: &str,
    depth: usize,
    total: &mut usize,
) -> Result<Vec<ObservedMenuItem>> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenuItemCount, GetMenuItemID, GetMenuState, GetMenuStringW, GetSubMenu, MF_BYPOSITION,
        MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_SEPARATOR,
    };
    if depth >= 4 || *total >= 128 {
        return Ok(Vec::new());
    }
    let count = unsafe { GetMenuItemCount(menu) }.max(0).min(64);
    let mut result = Vec::new();
    for position in 0..count {
        if *total >= 128 {
            break;
        }
        let mut buffer = vec![0u16; 512];
        let copied = unsafe {
            GetMenuStringW(
                menu,
                position as u32,
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                MF_BYPOSITION,
            )
        };
        let label = String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
            .replace('&', "")
            .trim()
            .to_string();
        let state = unsafe { GetMenuState(menu, position as u32, MF_BYPOSITION) };
        let separator = state & MF_SEPARATOR != 0;
        let path = if parent.is_empty() {
            label.clone()
        } else if label.is_empty() {
            parent.to_string()
        } else {
            format!("{parent}/{label}")
        };
        let submenu = unsafe { GetSubMenu(menu, position) };
        *total += 1;
        let children = if submenu.is_null() {
            Vec::new()
        } else {
            read_menu(submenu, &path, depth + 1, total)?
        };
        let command_id = unsafe { GetMenuItemID(menu, position) };
        result.push(ObservedMenuItem {
            label,
            path,
            command_id: (command_id != u32::MAX).then_some(command_id),
            enabled: state & (MF_DISABLED | MF_GRAYED) == 0,
            checked: state & MF_CHECKED != 0,
            separator,
            children,
        });
    }
    Ok(result)
}

#[cfg(windows)]
fn descendants(
    root: u32,
    parents: &std::collections::HashMap<u32, u32>,
) -> std::collections::HashSet<u32> {
    let mut result = std::collections::HashSet::from([root]);
    loop {
        let before = result.len();
        for (&process, &parent) in parents {
            if result.contains(&parent) {
                result.insert(process);
            }
        }
        if result.len() == before {
            return result;
        }
    }
}

#[cfg(not(windows))]
pub(super) fn capture_native_behavior(_root_process_id: u32) -> Result<NativeBehaviorSnapshot> {
    bail!("TAURI_NATIVE_WINDOWS_ONLY：Tauri 行为证据当前只支持 Windows 节点")
}
