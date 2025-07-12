#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::collections::BTreeSet;

use chrono::Local;
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
            Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
            Threading::{
                CREATE_NEW_CONSOLE, CREATE_NO_WINDOW, CreateMutexW, CreateProcessW,
                GetCurrentProcess, OpenProcess, PROCESS_ALL_ACCESS, PROCESS_INFORMATION,
                STARTUPINFOW,
            },
        },
        UI::Shell::{Common::COMDLG_FILTERSPEC, FileOpenDialog, IFileDialog, SIGDN_FILESYSPATH},
    },
    core::{HSTRING, Owned, PCWSTR, PWSTR, w},
};

use crate::{APP_NAME, SCRStruct};

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

pub fn get_path() -> Option<String> {
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

pub async fn save_log(mut logs: BTreeSet<String>) -> Result<(), String> {
    use tokio::{
        fs::File,
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    };

    let file_path = format!("{}.txt", Local::now().format("%Y-%m-%d"));
    if let Ok(file) = File::open(&file_path).await {
        let mut lines = BufReader::new(file).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            logs.insert(line);
        }
    }
    let mut file = File::create(&file_path)
        .await
        .map_err(|err| err.to_string())?;
    file.write_all(logs.into_iter().collect::<Vec<_>>().join("\r\n").as_bytes())
        .await
        .map_err(|err| err.to_string())?;

    Ok(())
}

pub fn get_owned_handle(pid: u32) -> Option<Owned<HANDLE>> {
    let owned_handle = unsafe { Owned::new(OpenProcess(PROCESS_ALL_ACCESS, false, pid).ok()?) };

    Some(owned_handle)
}

pub fn process_handles() -> Vec<SCRStruct> {
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
                    if !handle.is_invalid() && current.ImageName.Length > 0 {
                        if let Ok(name) = unsafe { current.ImageName.Buffer.to_string() } {
                            if name.eq_ignore_ascii_case("starcraft.exe") {
                                childs.push(SCRStruct::new(current.UniqueProcessId.0 as u32));
                            }
                        }
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

pub fn query_child(pid: u32, maybe_handle: Option<Owned<HANDLE>>) -> Option<String> {
    let mut buffer = Vec::new();
    let mut size = 0;
    let owned_handle = maybe_handle.unwrap_or(get_owned_handle(pid)?);

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
                    let process_handle_snapshot_information =
                        &*(buffer.as_ptr() as *const PROCESS_HANDLE_SNAPSHOT_INFORMATION);
                    let handles_ptr = process_handle_snapshot_information.Handles.as_ptr();
                    let number_of_handles = process_handle_snapshot_information.NumberOfHandles;

                    std::slice::from_raw_parts(handles_ptr, number_of_handles)
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

                                if p_object_info.Name.Length > 0 {
                                    let name = unsafe { p_object_info.Name.Buffer.to_string() };
                                    if name.is_ok_and(|name| {
                                        name.contains("Starcraft Check For Other Instances")
                                    }) && unsafe {
                                        DuplicateHandle(
                                            *owned_handle,
                                            handle_info.HandleValue,
                                            GetCurrentProcess(),
                                            &mut new_handle,
                                            0,
                                            false,
                                            DUPLICATE_CLOSE_SOURCE, // Close Source HANDLE
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
                                }
                                break;
                            }
                            _ => break,
                        } // match
                    } // loop
                } // for
                break;
            }
            _ => break,
        } // match
    } // loop

    None
}

pub fn run_scr(path: &str, args: &[&str]) -> Option<(u32, Owned<HANDLE>)> {
    let mut cmd = vec![path];
    cmd.extend_from_slice(args);
    let mut process_info = PROCESS_INFORMATION::default();
    let startup_info = STARTUPINFOW::default();

    let owned_handle = unsafe {
        CreateProcessW(
            None,
            Some(PWSTR(HSTRING::from(cmd.join(" ")).as_ptr() as *mut _)),
            None,
            None,
            false,
            CREATE_NO_WINDOW | CREATE_NEW_CONSOLE,
            None,
            None,
            &startup_info,
            &mut process_info,
        )
        .ok()?;
        CloseHandle(process_info.hThread).ok()?;
        Owned::new(process_info.hProcess)
    };

    Some((process_info.dwProcessId, owned_handle))
}
