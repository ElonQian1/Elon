#[cfg(windows)]
use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
};

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone)]
pub(super) struct NativeWindow {
    pub(super) handle: isize,
    pub(super) process_id: u32,
    pub(super) title: String,
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}

#[cfg(windows)]
pub(super) fn find_best_window(root_process_id: u32) -> Result<Option<NativeWindow>> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, BOOL, HWND, INVALID_HANDLE_VALUE, LPARAM, RECT},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, IsWindowVisible,
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        bail!("TAURI_PROCESS_SNAPSHOT_FAILED：无法枚举 Tauri 进程树");
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
    let mut windows = Vec::<NativeWindow>::new();
    struct Context<'a> {
        allowed: &'a HashSet<u32>,
        windows: &'a mut Vec<NativeWindow>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let context = &mut *(lparam as *mut Context<'_>);
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if !context.allowed.contains(&process_id) {
            return 1;
        }
        let length = GetWindowTextLengthW(hwnd).clamp(0, 1_000);
        let mut buffer = vec![0u16; length as usize + 1];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        let title = String::from_utf16_lossy(&buffer[..copied.max(0) as usize])
            .trim()
            .to_string();
        let mut rect: RECT = std::mem::zeroed();
        if title.is_empty() || GetWindowRect(hwnd, &mut rect) == 0 {
            return 1;
        }
        let width = (rect.right - rect.left).max(0) as u32;
        let height = (rect.bottom - rect.top).max(0) as u32;
        if width < 160 || height < 120 {
            return 1;
        }
        context.windows.push(NativeWindow {
            handle: hwnd as isize,
            process_id,
            title,
            left: rect.left,
            top: rect.top,
            width,
            height,
        });
        1
    }
    let mut context = Context {
        allowed: &allowed,
        windows: &mut windows,
    };
    unsafe {
        EnumWindows(Some(visit), &mut context as *mut Context<'_> as LPARAM);
    }
    windows.sort_by_key(|window| std::cmp::Reverse(window.width as u64 * window.height as u64));
    Ok(windows.into_iter().next())
}

#[cfg(not(windows))]
pub(super) fn find_best_window(_root_process_id: u32) -> Result<Option<NativeWindow>> {
    bail!("TAURI_NATIVE_WINDOWS_ONLY：Tauri 原生窗口证据当前只支持 Windows 节点")
}

#[cfg(windows)]
pub(super) fn capture_png(window: &NativeWindow, path: &std::path::Path) -> Result<Vec<u8>> {
    use windows_sys::Win32::{
        Graphics::Gdi::{
            CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
            GetWindowDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS,
        },
        Storage::Xps::PrintWindow,
    };
    const PW_RENDERFULLCONTENT: u32 = 0x0000_0002;
    let hwnd = window.handle as windows_sys::Win32::Foundation::HWND;
    let source = unsafe { GetWindowDC(hwnd) };
    if source.is_null() {
        bail!("TAURI_WINDOW_DC_FAILED：无法读取 Tauri 窗口表面");
    }
    let memory = unsafe { CreateCompatibleDC(source) };
    let bitmap =
        unsafe { CreateCompatibleBitmap(source, window.width as i32, window.height as i32) };
    if memory.is_null() || bitmap.is_null() {
        unsafe {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            if !memory.is_null() {
                DeleteDC(memory);
            }
            ReleaseDC(hwnd, source);
        }
        bail!("TAURI_WINDOW_BITMAP_FAILED：无法分配 Tauri 窗口截图缓冲区");
    }
    let previous = unsafe { SelectObject(memory, bitmap) };
    let rendered = unsafe { PrintWindow(hwnd, memory, PW_RENDERFULLCONTENT) };
    if rendered == 0 {
        cleanup_gdi(hwnd, source, memory, bitmap, previous);
        bail!("TAURI_PRINT_WINDOW_FAILED：原生窗口拒绝后台 PrintWindow 捕获");
    }
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: window.width as i32,
            biHeight: -(window.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { std::mem::zeroed() }
        },
        ..unsafe { std::mem::zeroed() }
    };
    let mut bgra = vec![0u8; window.width as usize * window.height as usize * 4];
    let lines = unsafe {
        GetDIBits(
            memory,
            bitmap,
            0,
            window.height,
            bgra.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    cleanup_gdi(hwnd, source, memory, bitmap, previous);
    if lines != window.height as i32 {
        bail!("TAURI_GET_DIBITS_FAILED：原生窗口像素读取不完整");
    }
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    image::save_buffer_with_format(
        path,
        &bgra,
        window.width,
        window.height,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .context("保存 Tauri 原生窗口 PNG 失败")?;
    std::fs::read(path).context("读取 Tauri 原生窗口 PNG 失败")
}

#[cfg(windows)]
fn cleanup_gdi(
    hwnd: windows_sys::Win32::Foundation::HWND,
    source: windows_sys::Win32::Graphics::Gdi::HDC,
    memory: windows_sys::Win32::Graphics::Gdi::HDC,
    bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP,
    previous: windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
) {
    use windows_sys::Win32::Graphics::Gdi::{DeleteDC, DeleteObject, ReleaseDC, SelectObject};
    unsafe {
        if !previous.is_null() {
            SelectObject(memory, previous);
        }
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(hwnd, source);
    }
}

#[cfg(not(windows))]
pub(super) fn capture_png(_window: &NativeWindow, _path: &std::path::Path) -> Result<Vec<u8>> {
    bail!("TAURI_NATIVE_WINDOWS_ONLY：Tauri 原生窗口证据当前只支持 Windows 节点")
}

#[cfg(windows)]
fn descendants(root: u32, parents: &HashMap<u32, u32>) -> HashSet<u32> {
    let mut result = HashSet::from([root]);
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
