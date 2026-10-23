//! Local credential protection. Unsupported platforms fail closed, never plaintext.

pub const AVAILABLE: bool = cfg!(windows);

#[cfg(windows)]
mod windows {
    use std::{ffi::c_void, ptr};

    #[repr(C)]
    struct Blob {
        len: u32,
        data: *mut u8,
    }

    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            input: *const Blob,
            description: *const u16,
            entropy: *const Blob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
        fn CryptUnprotectData(
            input: *const Blob,
            description: *mut *mut u16,
            entropy: *const Blob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
    }

    pub fn transform(value: &[u8], protect: bool) -> Result<Vec<u8>, &'static str> {
        if value.is_empty() || value.len() > 65536 {
            return Err("Invalid protected credential size");
        }
        let input = Blob {
            len: value.len() as u32,
            data: value.as_ptr().cast_mut(),
        };
        let mut output = Blob {
            len: 0,
            data: ptr::null_mut(),
        };
        // SAFETY: input points to a live bounded byte slice for the synchronous call.
        // DPAPI does not mutate it. Output is a LocalAlloc-owned blob; copied before
        // being wiped and freed with LocalFree. UI is forbidden (flag 1); machine-wide
        // protection is never enabled, so Windows binds the blob to CurrentUser.
        unsafe {
            let ok = if protect {
                CryptProtectData(
                    &input,
                    ptr::null(),
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    &mut output,
                )
            } else {
                CryptUnprotectData(
                    &input,
                    ptr::null_mut(),
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    &mut output,
                )
            };
            if ok == 0 {
                return Err("Windows could not unlock credentials for this user profile");
            }
            if output.data.is_null() {
                return Err("Windows returned an invalid credential buffer");
            }
            let result = std::slice::from_raw_parts(output.data, output.len as usize).to_vec();
            for index in 0..output.len as usize {
                ptr::write_volatile(output.data.add(index), 0);
            }
            LocalFree(output.data.cast());
            Ok(result)
        }
    }
}

/// Protects credentials using the current Windows user profile.
pub fn protect(value: &[u8]) -> Result<Vec<u8>, &'static str> {
    #[cfg(windows)]
    {
        windows::transform(value, true)
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Err("Credential storage is currently available on Windows only")
    }
}

/// Unlocks a protected blob; corrupted or foreign-profile blobs fail closed.
pub fn unprotect(value: &[u8]) -> Result<Vec<u8>, &'static str> {
    #[cfg(windows)]
    {
        windows::transform(value, false)
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Err("Credential storage is currently available on Windows only")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn credentials_fail_closed_for_corrupt_data() {
        assert!(super::unprotect(b"not a protected credential").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn current_user_roundtrip_is_not_plaintext_and_detects_tampering() {
        let secret = b"synthetic-test-credential-only";
        let mut protected = super::protect(secret).expect("DPAPI protection");
        assert!(!protected.windows(secret.len()).any(|part| part == secret));
        assert_eq!(super::unprotect(&protected).expect("DPAPI unlock"), secret);
        let last = protected.len() - 1;
        protected[last] ^= 1;
        assert!(super::unprotect(&protected).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_platform_never_writes_a_plaintext_fallback() {
        const { assert!(!super::AVAILABLE) };
        assert!(super::protect(b"synthetic-only").is_err());
    }
}
