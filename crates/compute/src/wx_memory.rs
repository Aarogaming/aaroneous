//! crates/compute/src/wx_memory.rs
//! RAII Executable Memory Manager enforcing strict W^X (Write XOR Execute) memory protection.
//! Provides two-phase lifecycle:
//! Phase 1 (Allocation & Write): Memory is Read-Write (RW), non-executable.
//! Phase 2 (Crystallization & Seal): Memory is transitioned to Read-Execute (RX), strictly non-writeable.
//! On Drop: Cleanly deallocates virtual memory pages back to the host OS.

use anyhow::{Result, anyhow};
use std::ptr::NonNull;

/// RAII Container for JIT-compiled native executable code pages
pub struct WxMemoryRegion {
    ptr: NonNull<u8>,
    size: usize,
    is_executable: bool,
}

unsafe impl Send for WxMemoryRegion {}
unsafe impl Sync for WxMemoryRegion {}

impl WxMemoryRegion {
    /// Allocates a new W^X memory region and writes machine code bytes in Phase 1 (RW),
    /// then transitions the page protection to Phase 2 (RX) before returning.
    pub fn from_machine_code(code: &[u8]) -> Result<Self> {
        if code.is_empty() {
            return Err(anyhow!("Cannot allocate empty executable memory region"));
        }

        // Align allocation size to page boundary (4KB standard)
        let page_size = 4096;
        let alloc_size = code.len().div_ceil(page_size) * page_size;

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Memory::{
                MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READ, PAGE_PROTECTION_FLAGS, PAGE_READWRITE,
                VirtualAlloc, VirtualProtect,
            };

            // SAFETY: `lpAddress = None` lets the OS pick the address;
            // `alloc_size` is non-zero and page-aligned, checked null below.
            let raw_ptr =
                unsafe { VirtualAlloc(None, alloc_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE) };

            if raw_ptr.is_null() {
                return Err(anyhow!(
                    "VirtualAlloc failed to allocate {} bytes",
                    alloc_size
                ));
            }

            let non_null = match NonNull::new(raw_ptr as *mut u8) {
                Some(p) => p,
                None => return Err(anyhow!("Allocated null pointer for executable region")),
            };

            // SAFETY: `code` is a valid `&[u8]`; `non_null` is the
            // `PAGE_READWRITE` region `VirtualAlloc` just committed with
            // `alloc_size >= code.len()`; the two cannot overlap.
            unsafe {
                std::ptr::copy_nonoverlapping(code.as_ptr(), non_null.as_ptr(), code.len());
            }

            // Phase 2: Lock page to Read-Execute (RX) - W^X enforcement
            let mut old_protect = PAGE_PROTECTION_FLAGS(0);
            // SAFETY: `raw_ptr`/`alloc_size` are the non-null base and size
            // just committed by `VirtualAlloc` above, wholly owned here.
            let protect_success =
                unsafe { VirtualProtect(raw_ptr, alloc_size, PAGE_EXECUTE_READ, &mut old_protect) };

            if let Err(e) = protect_success {
                // Clean up on protection failure
                use windows::Win32::System::Memory::{MEM_RELEASE, VirtualFree};
                // SAFETY: `raw_ptr` is the still-unreleased base address
                // this call reserved via `VirtualAlloc` above; `MEM_RELEASE`
                // requires that exact base address with size 0.
                unsafe {
                    let _ = VirtualFree(raw_ptr, 0, MEM_RELEASE);
                }
                return Err(anyhow!("VirtualProtect to PAGE_EXECUTE_READ failed: {e}"));
            }

            Ok(Self {
                ptr: non_null,
                size: alloc_size,
                is_executable: true,
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            use memmap2::MmapMut;

            let mut mmap = MmapMut::map_anon(alloc_size)
                .map_err(|e| anyhow!("mmap failed to allocate {} bytes: {e}", alloc_size))?;

            mmap[..code.len()].copy_from_slice(code);

            let exec_mmap = mmap
                .make_exec()
                .map_err(|e| anyhow!("mprotect to RX failed: {e}"))?;

            let ptr = NonNull::new(exec_mmap.as_ptr() as *mut u8)
                .ok_or_else(|| anyhow!("Mmap returned null pointer"))?;

            std::mem::forget(exec_mmap); // Managed via Drop

            Ok(Self {
                ptr,
                size: alloc_size,
                is_executable: true,
            })
        }
    }

    /// Returns the raw pointer to the executable memory block
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Returns the allocated page size in bytes
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns whether the allocated region is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Returns whether the memory region is currently in Executable (RX) mode
    pub fn is_executable(&self) -> bool {
        self.is_executable
    }

    /// Safely casts the executable memory region to a typed native function pointer
    ///
    /// # Safety
    /// The caller must ensure the function signature `F` matches the compiled calling convention
    /// and ABI of the machine code in this region.
    pub unsafe fn as_fn_ptr<F: Copy>(&self) -> F {
        // SAFETY: `self.ptr` is this region's executable page; per the
        // `# Safety` contract above, the caller guarantees `F`'s ABI
        // matches the compiled code.
        unsafe { std::mem::transmute_copy(&self.ptr.as_ptr()) }
    }
}

impl Drop for WxMemoryRegion {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Memory::{MEM_RELEASE, VirtualFree};
            // SAFETY: `self.ptr` was allocated via `VirtualAlloc` in
            // `from_machine_code`; `Drop::drop` runs at most once per value,
            // so this is the sole, still-valid release of that base address.
            unsafe {
                let _ = VirtualFree(self.ptr.as_ptr() as *mut _, 0, MEM_RELEASE);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // SAFETY: `self.ptr`/`self.size` describe the mapping made via
            // `MmapMut::map_anon` and `mem::forget`-ten in
            // `from_machine_code`; `Drop::drop` runs at most once.
            unsafe {
                libc::munmap(self.ptr.as_ptr() as *mut _, self.size);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wx_memory_allocation_and_protection() {
        // Simple return instruction (e.g. x86_64 ret = 0xC3)
        let code = [0xC3u8];
        let region = WxMemoryRegion::from_machine_code(&code).unwrap();

        assert!(region.is_executable());
        assert!(region.len() >= 4096);
        assert!(!region.as_ptr().is_null());

        // Test casting to C-calling convention fn()
        type RetFn = unsafe extern "C" fn();
        // SAFETY: `code` is the single-byte x86_64 `ret` instruction (0xC3),
        // taking no arguments and returning nothing, matching this ABI.
        let ret_fn: RetFn = unsafe { region.as_fn_ptr() };
        // SAFETY: `ret_fn` points into `region`, a live RX page holding
        // only the `ret` opcode; calling it with no args just returns.
        unsafe {
            ret_fn();
        }
    }
}
