#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use chrono::Local;
use std::{
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
    os::windows::ffi::OsStrExt,
};
use windows::{
    Wdk::{
        Foundation::{NtQueryObject, OBJECT_INFORMATION_CLASS, OBJECT_NAME_INFORMATION},
        System::{
            SystemInformation::{NtQuerySystemInformation, SystemProcessInformation},
            Threading::{NtQueryInformationProcess, ProcessHandleInformation},
        },
    },
    Win32::{
        Foundation::{
            CloseHandle, DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, DuplicateHandle,
            ERROR_ALREADY_EXISTS, GetLastError, HANDLE, STATUS_INFO_LENGTH_MISMATCH,
            STATUS_SUCCESS, UNICODE_STRING,
        },
        System::{
            Com::{
                CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
                CoUninitialize,
            },
            Threading::{
                CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CreateMutexW, CreateProcessW,
                GetCurrentProcess, IsWow64Process, OpenProcess, PROCESS_DUP_HANDLE,
                PROCESS_INFORMATION, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, STARTUPINFOW,
                TerminateProcess,
            },
        },
        UI::Shell::{Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileDialog, SIGDN_FILESYSPATH},
    },
    core::{BOOL, HSTRING, Owned, PCWSTR, PWSTR, w},
};

use crate::APP_NAME;

#[repr(C)]
#[derive(Debug)]
struct SYSTEM_PROCESS_INFORMATION {
    pub NextEntryOffset: u32,
    pub NumberOfThreads: u32,
    pub Reserved1: [u8; 48],
    pub ImageName: UNICODE_STRING,
    pub BasePriority: i32,
    pub UniqueProcessId: HANDLE,
}

#[repr(C)]
#[derive(Debug)]
struct PROCESS_HANDLE_SNAPSHOT_INFORMATION {
    pub NumberOfHandles: usize,
    pub Reserved: usize,
    pub Handles: [PROCESS_HANDLE_TABLE_ENTRY_INFO; 0],
}

#[repr(C)]
#[derive(Debug)]
struct PROCESS_HANDLE_TABLE_ENTRY_INFO {
    pub HandleValue: HANDLE,
    pub HandleCount: usize,
    pub PointerCount: usize,
    pub GrantedAccess: u32,
    pub ObjectTypeIndex: u32,
    pub HandleAttributes: u32,
    pub Reserved: u32,
}

const ObjectNameInformation: OBJECT_INFORMATION_CLASS = OBJECT_INFORMATION_CLASS(1i32);

pub fn get_mutex() -> bool {
    unsafe {
        CreateMutexW(None, false, PCWSTR(HSTRING::from(APP_NAME).as_ptr())).is_ok()
            && GetLastError() != ERROR_ALREADY_EXISTS
    }
}

pub async fn get_path_async() -> Option<String> {
    tokio::task::spawn_blocking(|| {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let path = get_path();
        unsafe {
            CoUninitialize();
        }
        path
    })
    .await
    .unwrap_or(None)
}

fn get_path() -> Option<String> {
    unsafe {
        let dialog =
            CoCreateInstance::<_, IFileDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
        dialog.SetTitle(w!("스타크래프트 실행 파일 선택")).ok()?;
        dialog
            .SetFileTypes(&[COMDLG_FILTERSPEC {
                pszName: w!("StarCraft.exe"),
                pszSpec: w!("StarCraft.exe"),
            }])
            .ok()?;
        dialog.Show(None).ok()?;
        dialog
            .GetResult()
            .and_then(|item| item.GetDisplayName(SIGDN_FILESYSPATH))
            .ok()
            .and_then(|item| item.to_string().ok().filter(|e| !e.is_empty()))
    }
}

pub trait StringExt {
    fn as_log(&self) -> String;
}

impl StringExt for str {
    fn as_log(&self) -> String {
        format!("[{}] {}", Local::now().format("%H:%M:%S%.3f"), self)
    }
}

pub async fn save_log(mut logs: BTreeSet<String>) -> Result<BTreeSet<String>, String> {
    use tokio::{
        fs::File,
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    };

    let file_path = format!("{}.txt", Local::now().format("%Y-%m-%d"));
    if let Ok(file) = File::open(&file_path).await {
        let mut lines = BufReader::new(file).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            logs.insert(line);
        }
    }
    let file = File::create(&file_path)
        .await
        .map_err(|err| err.to_string())?;
    let mut writer = BufWriter::new(file);
    for (i, log) in logs.iter().enumerate() {
        if i > 0 {
            writer
                .write_all(b"\r\n")
                .await
                .map_err(|err| err.to_string())?;
        }
        writer
            .write_all(log.as_bytes())
            .await
            .map_err(|err| err.to_string())?;
    }
    writer.flush().await.map_err(|err| err.to_string())?; // 남은 버퍼 밀어내기

    Ok(logs)
}

fn get_owned_handle(pid: u32) -> Option<Owned<HANDLE>> {
    unsafe {
        Owned::new(OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_DUP_HANDLE, false, pid).ok()?)
    }
    .into()
}

fn get_terminate_handle(pid: u32) -> Option<Owned<HANDLE>> {
    unsafe { Owned::new(OpenProcess(PROCESS_TERMINATE, false, pid).ok()?) }.into()
}

fn process_handles() -> Vec<u32> {
    let mut process_list_size: u32 = 0;
    let mut processes = Vec::new();
    let mut childs = Vec::new();
    loop {
        let status = unsafe {
            NtQuerySystemInformation(
                SystemProcessInformation,
                processes.as_mut_ptr() as _,
                processes.len() as _,
                &mut process_list_size,
            )
        };
        match status {
            STATUS_INFO_LENGTH_MISMATCH => processes.resize(process_list_size as _, 0u8),
            STATUS_SUCCESS => {
                let mut current_offset = 0;
                loop {
                    let current = unsafe {
                        &*(processes.as_ptr().add(current_offset)
                            as *const SYSTEM_PROCESS_INFORMATION)
                    };
                    let handle = current.UniqueProcessId;
                    if !handle.is_invalid()
                        && current.ImageName.Length > 0
                        && let Ok(name) = unsafe { current.ImageName.Buffer.to_string() }
                        && name.eq_ignore_ascii_case("starcraft.exe")
                    {
                        childs.push(current.UniqueProcessId.0 as u32);
                    }
                    if current.NextEntryOffset == 0 {
                        break;
                    }
                    current_offset += current.NextEntryOffset as usize;
                }
                break;
            }
            _ => break,
        }
    }
    childs
}

fn query_child(pid: u32, maybe_handle: Option<Owned<HANDLE>>) -> Option<String> {
    let mut buffer = Vec::new();
    let mut size = 0;
    let owned_handle = maybe_handle.or_else(|| get_owned_handle(pid))?;

    loop {
        let status = unsafe {
            NtQueryInformationProcess(
                *owned_handle,
                ProcessHandleInformation,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut size,
            )
        };
        match status {
            STATUS_INFO_LENGTH_MISMATCH => buffer.resize(size as _, 0u8),
            STATUS_SUCCESS => {
                let handles_slice = unsafe {
                    let info = &*(buffer.as_ptr() as *const PROCESS_HANDLE_SNAPSHOT_INFORMATION);
                    std::slice::from_raw_parts(info.Handles.as_ptr(), info.NumberOfHandles)
                };

                for handle_info in handles_slice {
                    let mut new_handle = HANDLE::default();
                    if unsafe {
                        DuplicateHandle(
                            *owned_handle,
                            handle_info.HandleValue,
                            GetCurrentProcess(),
                            &mut new_handle,
                            0,
                            false,
                            DUPLICATE_SAME_ACCESS,
                        )
                    }
                    .is_err()
                    {
                        continue;
                    }

                    let mut object_buf: Vec<u8> = Vec::new();
                    let mut object_len: u32 = 0;

                    loop {
                        let status = unsafe {
                            NtQueryObject(
                                Some(new_handle),
                                ObjectNameInformation,
                                Some(object_buf.as_mut_ptr() as *mut _),
                                object_buf.len() as u32,
                                Some(&mut object_len),
                            )
                        };
                        match status {
                            STATUS_INFO_LENGTH_MISMATCH => {
                                object_buf.resize(object_len as usize, 0u8)
                            }
                            STATUS_SUCCESS => {
                                if unsafe { CloseHandle(new_handle) }.is_err() {
                                    break;
                                };
                                let p_object_info = unsafe {
                                    &*(object_buf.as_ptr() as *const OBJECT_NAME_INFORMATION)
                                };

                                if p_object_info.Name.Length > 0
                                    && let Ok(name) =
                                        unsafe { p_object_info.Name.Buffer.to_string() }
                                    && name.contains("Starcraft Check For Other Instances")
                                    && unsafe {
                                        DuplicateHandle(
                                            *owned_handle,
                                            handle_info.HandleValue,
                                            GetCurrentProcess(),
                                            &mut new_handle,
                                            0,
                                            false,
                                            DUPLICATE_CLOSE_SOURCE,
                                        )
                                        .and_then(|_| CloseHandle(new_handle))
                                    }
                                    .is_ok()
                                {
                                    return Some(
                                        format!(
                                            "Closed {:?} for StarCraft.exe (PID: {})",
                                            handle_info.HandleValue, pid
                                        )
                                        .as_log(),
                                    );
                                }
                                break;
                            }
                            _ => break,
                        }
                    }
                }
                break;
            }
            _ => break,
        }
    }
    None
}

// BTreeMap의 소유권을 완전히 가져와서 작업한 뒤 다시 반환합니다. 락(Lock) 없음!
pub async fn check_processes_worker(
    mut childs: BTreeMap<u32, bool>,
) -> (BTreeMap<u32, bool>, Vec<String>) {
    tokio::task::spawn_blocking(move || {
        let current_pids = process_handles();
        let mut logs = Vec::new();

        for pid in current_pids {
            childs.entry(pid).or_insert(false);
        }

        childs.retain(|&pid, is_processed| {
            if *is_processed {
                if get_owned_handle(pid).is_none() {
                    logs.push(format!("Invalid PID: {}", pid).as_log());
                    false // 삭제
                } else {
                    true // 유지
                }
            } else if let Some(log) = query_child(pid, None) {
                *is_processed = true;
                logs.push(log);
                true
            } else {
                true
            }
        });

        (childs, logs)
    })
    .await
    .unwrap_or_else(|_| (BTreeMap::new(), Vec::new()))
}

fn exe_architecture(path: &std::path::Path) -> Option<&'static str> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(path).ok()?;
    let mut dos_header = [0u8; 64];
    file.read_exact(&mut dos_header).ok()?;
    if &dos_header[0..2] != b"MZ" {
        return None;
    }

    let pe_offset = u32::from_le_bytes(dos_header[0x3c..0x40].try_into().ok()?) as u64;
    file.seek(SeekFrom::Start(pe_offset)).ok()?;

    let mut pe_header = [0u8; 6];
    file.read_exact(&mut pe_header).ok()?;
    if &pe_header[0..4] != b"PE\0\0" {
        return None;
    }

    match u16::from_le_bytes([pe_header[4], pe_header[5]]) {
        0x014c => Some("32비트 (x86)"),
        0x8664 => Some("64비트 (x64)"),
        0xaa64 => Some("ARM64"),
        _ => Some("알 수 없는 아키텍처"),
    }
}

fn process_architecture(process: HANDLE) -> Option<&'static str> {
    let mut is_wow64 = BOOL(0);
    unsafe { IsWow64Process(process, &mut is_wow64) }.ok()?;

    Some(if is_wow64.as_bool() {
        "32비트 (WOW64)"
    } else {
        "64비트"
    })
}

pub async fn run_scr_async(path: String, args: Vec<String>) -> Vec<String> {
    tokio::task::spawn_blocking(move || {
        let mut logs = Vec::new();

        let exe_path = std::path::Path::new(&path);
        if !exe_path.is_file() {
            logs.push(format!("실행 파일을 찾을 수 없습니다: {}", path).as_log());
            return logs;
        }

        // 어떤 아키텍처의 실행 파일을 선택했는지 로그로 남긴다.
        logs.push(
            format!(
                "실행 파일: {} [{}]",
                path,
                exe_architecture(exe_path).unwrap_or("알 수 없음")
            )
            .as_log(),
        );

        // CreateProcessW가 lpCommandLine을 수정할 수 있으므로 쓰기 가능한 버퍼를 넘긴다.
        // 경로를 따옴표로 감싸 공백이 포함된 경로(Program Files 등)를 안전하게 처리한다.
        let mut command_line: Vec<u16> = std::iter::once(format!("\"{}\"", path))
            .chain(args)
            .collect::<Vec<_>>()
            .join(" ")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let application: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let current_dir: Vec<u16> = std::path::Path::new(&path)
            .parent()
            .filter(|dir| !dir.as_os_str().is_empty())
            .map(|dir| dir.as_os_str().encode_wide().chain(Some(0)).collect())
            .unwrap_or_default();

        let mut process_info = PROCESS_INFORMATION::default();
        let startup_info = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };

        let created = unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command_line.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_NO_WINDOW | CREATE_NEW_CONSOLE,
                None,
                if current_dir.is_empty() {
                    PCWSTR::null()
                } else {
                    PCWSTR(current_dir.as_ptr())
                },
                &startup_info,
                &mut process_info,
            )
        };

        if let Err(err) = created {
            logs.push(
                format!(
                    "StarCraft.exe 실행 실패 (경로: {}): {} [{}]",
                    path,
                    err.message(),
                    err.code()
                )
                .as_log(),
            );
            return logs;
        }

        let pid = process_info.dwProcessId;
        logs.push(format!("StarCraft.exe 실행 성공 (PID: {})", pid).as_log());
        if let Some(architecture) = process_architecture(process_info.hProcess) {
            logs.push(format!("실행된 프로세스 아키텍처: {}", architecture).as_log());
        }
        let _ = unsafe { CloseHandle(process_info.hThread) };

        // 게임이 뮤텍스를 만드는 데 시간이 걸리므로 잠시 재시도한다.
        let mut owned_handle = Some(unsafe { Owned::new(process_info.hProcess) });
        for _ in 0..5 {
            if let Some(log) = query_child(pid, owned_handle.take()) {
                logs.push(log);
                return logs;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        logs.push(
            format!(
                "PID {}의 인스턴스 뮤텍스를 아직 찾지 못했습니다. 주기적 검사에서 계속 시도합니다.",
                pid
            )
            .as_log(),
        );
        logs
    })
    .await
    .unwrap_or_else(|err| vec![format!("StarCraft.exe 실행 작업 실패: {}", err).as_log()])
}

pub async fn kill_processes_async(
    mut childs: BTreeMap<u32, bool>,
) -> (BTreeMap<u32, bool>, Vec<String>) {
    tokio::task::spawn_blocking(move || {
        let mut logs = Vec::new();

        for &pid in childs.keys() {
            if let Some(handle) = get_terminate_handle(pid)
                && unsafe { TerminateProcess(*handle, 0) }.is_ok()
            {
                logs.push(format!("Successfully terminated process with PID {}", pid).as_log());
            }
        }
        childs.clear();
        (childs, logs)
    })
    .await
    .unwrap_or_else(|_| (BTreeMap::new(), Vec::new()))
}
