pub mod xattr {
    #[cfg(target_os = "windows")]
    use anyhow::{
        Result,
        anyhow,
    };

    /// Implementing xattr-like feature on Windows using Alternate Data Streams(ADS)
    /// ADS only works on NTFS filesystem, and maybe removed in specific operations.
    /// https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/e2b19412-a925-4360-b009-86e3b8a020c8
    #[cfg(target_os = "windows")]
    pub fn get_xattr(path: &std::path::Path, attr_name: &str) -> Result<String> {
        use std::{
            ffi::OsStr,
            io,
            os::windows::ffi::OsStrExt,
            str,
        };
        use windows::{
            Win32::{
                Foundation::{
                    CloseHandle,
                    INVALID_HANDLE_VALUE,
                },
                Storage::FileSystem::{
                    BY_HANDLE_FILE_INFORMATION,
                    CreateFileW,
                    FILE_ATTRIBUTE_NORMAL,
                    GetFileInformationByHandle,
                    OPEN_EXISTING,
                    ReadFile,
                },
            },
            core::{
                Error,
                PCWSTR,
            },
        };

        let full_path = format!("{}:{}", path.display(), attr_name);

        let wide_path: Vec<u16> = OsStr::new(&full_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_path_pcwstr = PCWSTR::from_raw(wide_path.as_ptr());

        unsafe {
            let handle = CreateFileW(
                wide_path_pcwstr,
                2147483648u32,
                windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(0),
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?;

            if handle == INVALID_HANDLE_VALUE {
                let err = Error::from(io::Error::last_os_error());
                if err.code().0 as u32 == 2 {
                    return Err(anyhow!(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Attribute {} not found", attr_name),
                    )));
                }
                return Err(anyhow!(err));
            }

            let mut file_info = BY_HANDLE_FILE_INFORMATION::default();
            GetFileInformationByHandle(handle, &mut file_info)?;

            let file_size =
                (file_info.nFileSizeHigh as u64) << 32 | (file_info.nFileSizeLow as u64);

            let mut buffer = vec![0u8; file_size as usize];
            let mut bytes_read: u32 = 0;

            ReadFile(handle, Some(&mut buffer), Some(&mut bytes_read), None)?;
            CloseHandle(handle)?;

            if bytes_read != file_size as u32 {
                return Err(anyhow!(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to read entire stream",
                )));
            }

            match str::from_utf8(&buffer) {
                Ok(s) => Ok(s.to_string()),
                Err(_) => Err(anyhow!(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Stream data is not valid UTF-8",
                ))),
            }
        }
    }

    #[cfg(target_os = "windows")]
    pub fn set_xattr(path: &std::path::Path, attr_name: &str, value: String) -> Result<()> {
        use std::{
            ffi::OsStr,
            io,
            os::windows::ffi::OsStrExt,
        };
        use windows::{
            Win32::{
                Foundation::{
                    CloseHandle,
                    INVALID_HANDLE_VALUE,
                },
                Storage::FileSystem::{
                    CREATE_ALWAYS,
                    CreateFileW,
                    FILE_ATTRIBUTE_NORMAL,
                    WriteFile,
                },
            },
            core::PCWSTR,
        };

        let full_path = format!("{}:{}", path.display(), attr_name);

        let wide_path: Vec<u16> = OsStr::new(&full_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_path_pcwstr = PCWSTR::from_raw(wide_path.as_ptr());

        unsafe {
            let handle = CreateFileW(
                wide_path_pcwstr,
                1073741824u32,
                windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(0),
                None,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )?;

            if handle == INVALID_HANDLE_VALUE {
                return Err(anyhow!(io::Error::last_os_error()));
            }

            let buffer = value.as_bytes();
            let mut bytes_written: u32 = 0;

            WriteFile(handle, Some(buffer), Some(&mut bytes_written), None)?;
            CloseHandle(handle)?;

            if bytes_written != buffer.len() as u32 {
                return Err(anyhow!(io::Error::new(
                    io::ErrorKind::Other,
                    "Failed to write entire stream",
                )));
            }

            Ok(())
        }
    }
}
