//! Bindings for the Unicorn emulator.
//!
//!
//!
//! # Example use
//!
//! ```rust
//!
//! use unicornafl::Unicorn;
//! use unicornafl::arm::Register;
//! use unicornafl::consts::{Arch, Mode, Permission, SECOND_SCALE};
//!
//! let arm_code32 = [0x17, 0x00, 0x40, 0xe2]; // sub r0, #23
//!
//! let mut emu = Unicorn::new(Arch::ARM, Mode::LITTLE_ENDIAN).expect("failed to initialize Unicorn instance");
//! emu.mem_map(0x1000, 0x4000, Permission::ALL).expect("failed to map code page");
//! emu.mem_write(0x1000, &arm_code32).expect("failed to write instructions");
//!
//! emu.reg_write(Register::R0, 123).expect("failed write R0");
//! emu.reg_write(Register::R5, 1337).expect("failed write R5");
//!
//! emu.emu_start(0x1000, (0x1000 + arm_code32.len()) as u64, 10 * SECOND_SCALE, 1000).unwrap();
//! assert_eq!(emu.reg_read(Register::R0), Ok(100));
//! assert_eq!(emu.reg_read(Register::R5), Ok(1337));
//! ```
//!

pub mod afl;
pub mod arm;
pub mod arm64;
pub mod consts;
pub mod m68k;
pub mod mips;
pub mod ppc;
pub mod riscv;
pub mod sparc;
pub mod x86;

mod ffi;

use crate::consts::{uc_error, Arch, HookType, MemRegion, MemType, Mode, Permission, Query};
use crate::ffi::uc_handle;

use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::fmt;
use std::mem;
use std::ptr;
use std::rc::Rc;
use tinyvec::ArrayVec;

#[derive(Debug)]
pub struct Context {
    context: ffi::uc_context,
}

impl Context {
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        !self.context.is_null()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if self.is_initialized() {
            unsafe {
                ffi::uc_context_free(self.context);
            }
        }
        self.context = ptr::null_mut();
    }
}

pub struct MmioCallbackScope<'a> {
    pub regions: Vec<(u64, usize)>,
    pub read_callback: Option<Box<dyn ffi::IsUcHook<'a> + 'a>>,
    pub write_callback: Option<Box<dyn ffi::IsUcHook<'a> + 'a>>,
}

impl<'a> MmioCallbackScope<'a> {
    fn has_regions(&self) -> bool {
        self.regions.len() > 0
    }

    fn unmap(&mut self, begin: u64, size: usize) {
        let end = begin + size as u64;
        self.regions = self
            .regions
            .iter()
            .flat_map(|(b, s)| {
                let e = b + *s as u64;
                let mut v = ArrayVec::<[(u64, usize); 2]>::new();
                if begin > *b {
                    if begin >= e {
                        // The unmapped region is completely after this region
                    } else {
                        if end >= e {
                            // The unmapped region overlaps with the end of this region
                            v.push((*b, (begin - *b) as usize));
                        } else {
                            // The unmapped region is in the middle of this region
                            let second_b = end + 1;
                            v.push((*b, (begin - *b) as usize));
                            v.push((second_b, (e - second_b) as usize));
                        }
                    }
                } else {
                    if end > *b {
                        if end >= e {
                            // The unmapped region completely contains this region
                        } else {
                            // The unmapped region overlaps with the start of this region
                            v.push((end, (e - end) as usize));
                        }
                    } else {
                        // The unmapped region is completely before this region
                        v.push((*b, *s));
                    }
                };
                v
            })
            .collect();
    }
}

pub struct UnicornInner<'a, D> {
    pub uc: uc_handle,
    pub arch: Arch,
    /// to keep ownership over the hook for this uc instance's lifetime
    pub hooks: Vec<(ffi::uc_hook, Box<dyn ffi::IsUcHook<'a> + 'a>)>,
    /// To keep ownership over the mmio callbacks for this uc instance's lifetime
    pub mmio_callbacks: Vec<MmioCallbackScope<'a>>,
    pub data: D,
}

/// Drop UC
impl<'a, D> Drop for UnicornInner<'a, D> {
    fn drop(&mut self) {
        if !self.uc.is_null() {
            unsafe { ffi::uc_close(self.uc) };
        }
        self.uc = ptr::null_mut();
    }
}

/// A Unicorn emulator instance.
pub struct Unicorn<'a, D: 'a> {
    inner: Rc<UnsafeCell<UnicornInner<'a, D>>>,
}

impl<'a> Unicorn<'a, ()> {
    /// Create a new instance of the unicorn engine for the specified architecture
    /// and hardware mode.
    pub fn new(arch: Arch, mode: Mode) -> Result<Unicorn<'a, ()>, uc_error> {
        Self::new_with_data(arch, mode, ())
    }
}

impl<'a, D> Unicorn<'a, D>
where
    D: 'a,
{
    /// Create a new instance of the unicorn engine for the specified architecture
    /// and hardware mode.
    pub fn new_with_data(arch: Arch, mode: Mode, data: D) -> Result<Unicorn<'a, D>, uc_error> {
        let mut handle = ptr::null_mut();
        let err = unsafe { ffi::uc_open(arch, mode, &mut handle) };
        if err == uc_error::OK {
            Ok(Unicorn {
                inner: Rc::new(UnsafeCell::from(UnicornInner {
                    uc: handle,
                    arch,
                    data,
                    hooks: vec![],
                    mmio_callbacks: vec![],
                })),
            })
        } else {
            Err(err)
        }
    }
}

impl<'a, D> fmt::Debug for Unicorn<'a, D> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Unicorn {{ uc: {:p} }}", self.inner().uc)
    }
}

impl<'a, D> Unicorn<'a, D> {
    fn inner(&self) -> &UnicornInner<'a, D> {
        unsafe { self.inner.get().as_ref().unwrap() }
    }

    fn inner_mut(&mut self) -> &mut UnicornInner<'a, D> {
        unsafe { self.inner.get().as_mut().unwrap() }
    }

    /// Return whatever data was passed during initialization.
    ///
    #[must_use]
    pub fn get_data(&self) -> &D {
        &self.inner().data
    }

    /// Return a mutable reference to whatever data was passed during initialization.
    #[must_use]
    pub fn get_data_mut(&mut self) -> &mut D {
        &mut self.inner_mut().data
    }

    /// Return the architecture of the current emulator.
    #[must_use]
    pub fn get_arch(&self) -> Arch {
        self.inner().arch
    }

    /// Returns a vector with the memory regions that are mapped in the emulator.
    pub fn mem_regions(&self) -> Result<Vec<MemRegion>, uc_error> {
        let mut nb_regions: u32 = 0;
        let p_regions: *const MemRegion = ptr::null_mut();
        let err = unsafe { ffi::uc_mem_regions(self.inner().uc, &p_regions, &mut nb_regions) };
        if err == uc_error::OK {
            let mut regions = Vec::new();
            for i in 0..nb_regions {
                regions.push(unsafe { mem::transmute_copy(&*p_regions.add(i as usize)) });
            }
            unsafe { libc::free(p_regions as _) };
            Ok(regions)
        } else {
            Err(err)
        }
    }

    /// Read a range of bytes from memory at the specified address.
    pub fn mem_read(&self, address: u64, buf: &mut [u8]) -> Result<(), uc_error> {
        let err =
            unsafe { ffi::uc_mem_read(self.inner().uc, address, buf.as_mut_ptr(), buf.len()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Return a range of bytes from memory at the specified address as vector.
    pub fn mem_read_as_vec(&self, address: u64, size: usize) -> Result<Vec<u8>, uc_error> {
        let mut buf = vec![0; size];
        let err = unsafe { ffi::uc_mem_read(self.inner().uc, address, buf.as_mut_ptr(), size) };
        if err == uc_error::OK {
            Ok(buf)
        } else {
            Err(err)
        }
    }

    pub fn mem_write(&mut self, address: u64, bytes: &[u8]) -> Result<(), uc_error> {
        let err =
            unsafe { ffi::uc_mem_write(self.inner().uc, address, bytes.as_ptr(), bytes.len()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map an existing memory region in the emulator at the specified address.
    ///
    /// # Safety
    ///
    /// This function is marked unsafe because it is the responsibility of the caller to
    /// ensure that `size` matches the size of the passed buffer, an invalid `size` value will
    /// likely cause a crash in unicorn.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    ///
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    ///
    /// `ptr` is a pointer to the provided memory region that will be used by the emulator.
    pub unsafe fn mem_map_ptr(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
        ptr: *mut c_void,
    ) -> Result<(), uc_error> {
        let err = ffi::uc_mem_map_ptr(self.inner().uc, address, size, perms.bits(), ptr);
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map a memory region in the emulator at the specified address.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_map(
        &mut self,
        address: u64,
        size: libc::size_t,
        perms: Permission,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_map(self.inner().uc, address, size, perms.bits()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map in am MMIO region backed by callbacks.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mmio_map<R: 'a, W: 'a>(
        &mut self,
        address: u64,
        size: libc::size_t,
        read_callback: Option<R>,
        write_callback: Option<W>,
    ) -> Result<(), uc_error>
    where
        R: FnMut(&mut Unicorn<D>, u64, usize) -> u64,
        W: FnMut(&mut Unicorn<D>, u64, usize, u64),
    {
        let mut read_data = read_callback.map(|c| {
            Box::new(ffi::UcHook {
                callback: c,
                uc: Unicorn {
                    inner: self.inner.clone(),
                },
            })
        });
        let mut write_data = write_callback.map(|c| {
            Box::new(ffi::UcHook {
                callback: c,
                uc: Unicorn {
                    inner: self.inner.clone(),
                },
            })
        });

        let err = unsafe {
            ffi::uc_mmio_map(
                self.inner().uc,
                address,
                size,
                ffi::mmio_read_callback_proxy::<D, R> as _,
                match read_data {
                    Some(ref mut d) => d.as_mut() as *mut _ as _,
                    None => ptr::null_mut(),
                },
                ffi::mmio_write_callback_proxy::<D, W> as _,
                match write_data {
                    Some(ref mut d) => d.as_mut() as *mut _ as _,
                    None => ptr::null_mut(),
                },
            )
        };

        if err == uc_error::OK {
            let rd = read_data.map(|c| c as Box<dyn ffi::IsUcHook>);
            let wd = write_data.map(|c| c as Box<dyn ffi::IsUcHook>);
            self.inner_mut().mmio_callbacks.push(MmioCallbackScope {
                regions: vec![(address, size)],
                read_callback: rd,
                write_callback: wd,
            });

            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map in a read-only MMIO region backed by a callback.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mmio_map_ro<F: 'a>(
        &mut self,
        address: u64,
        size: libc::size_t,
        callback: F,
    ) -> Result<(), uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u64, usize) -> u64,
    {
        self.mmio_map(
            address,
            size,
            Some(callback),
            None::<fn(&mut Unicorn<D>, u64, usize, u64)>,
        )
    }

    /// Map in a write-only MMIO region backed by a callback.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mmio_map_wo<F: 'a>(
        &mut self,
        address: u64,
        size: libc::size_t,
        callback: F,
    ) -> Result<(), uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u64, usize, u64),
    {
        self.mmio_map(
            address,
            size,
            None::<fn(&mut Unicorn<D>, u64, usize) -> u64>,
            Some(callback),
        )
    }

    /// Unmap a memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_unmap(&mut self, address: u64, size: libc::size_t) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_unmap(self.inner().uc, address, size) };

        self.mmio_unmap(address, size);

        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    fn mmio_unmap(&mut self, address: u64, size: libc::size_t) {
        for scope in self.inner_mut().mmio_callbacks.iter_mut() {
            scope.unmap(address, size);
        }
        self.inner_mut()
            .mmio_callbacks
            .retain(|scope| scope.has_regions());
    }

    /// Set the memory permissions for an existing memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_protect(
        &mut self,
        address: u64,
        size: libc::size_t,
        perms: Permission,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_protect(self.inner().uc, address, size, perms.bits()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Write an unsigned value from a register.
    pub fn reg_write<T: Into<i32>>(&mut self, regid: T, value: u64) -> Result<(), uc_error> {
        let err =
            unsafe { ffi::uc_reg_write(self.inner().uc, regid.into(), &value as *const _ as _) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Write variable sized values into registers.
    ///
    /// The user has to make sure that the buffer length matches the register size.
    /// This adds support for registers >64 bit (GDTR/IDTR, XMM, YMM, ZMM (x86); Q, V (arm64)).
    pub fn reg_write_long<T: Into<i32>>(&self, regid: T, value: &[u8]) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_reg_write(self.inner().uc, regid.into(), value.as_ptr() as _) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Read an unsigned value from a register.
    ///
    /// Not to be used with registers larger than 64 bit.
    pub fn reg_read<T: Into<i32>>(&self, regid: T) -> Result<u64, uc_error> {
        let mut value: u64 = 0;
        let err =
            unsafe { ffi::uc_reg_read(self.inner().uc, regid.into(), &mut value as *mut u64 as _) };
        if err == uc_error::OK {
            Ok(value)
        } else {
            Err(err)
        }
    }

    /// Read 128, 256 or 512 bit register value into heap allocated byte array.
    ///
    /// This adds safe support for registers >64 bit (GDTR/IDTR, XMM, YMM, ZMM, ST (x86); Q, V (arm64)).
    pub fn reg_read_long<T: Into<i32>>(&self, regid: T) -> Result<ArrayVec<[u8; 64]>, uc_error> {
        let curr_reg_id = regid.into();
        let curr_arch = self.get_arch();
        let zeros = [0u8; 64];

        let mut value = if curr_arch == Arch::X86 {
            if curr_reg_id >= x86::Register::XMM0 as i32
                && curr_reg_id <= x86::Register::XMM31 as i32
            {
                ArrayVec::from_array_len(zeros, 16)
            } else if curr_reg_id >= x86::Register::YMM0 as i32
                && curr_reg_id <= x86::Register::YMM31 as i32
            {
                ArrayVec::from_array_len(zeros, 32)
            } else if curr_reg_id >= x86::Register::ZMM0 as i32
                && curr_reg_id <= x86::Register::ZMM31 as i32
            {
                ArrayVec::from(zeros)
            } else if curr_reg_id == x86::Register::GDTR as i32
                || curr_reg_id == x86::Register::IDTR as i32
                || (curr_reg_id >= x86::Register::ST0 as i32
                    && curr_reg_id <= x86::Register::ST7 as i32)
            {
                ArrayVec::from_array_len(zeros, 10)
            } else {
                return Err(uc_error::ARG);
            }
        } else if curr_arch == Arch::ARM64 {
            if (curr_reg_id >= arm64::Register::Q0 as i32
                && curr_reg_id <= arm64::Register::Q31 as i32)
                || (curr_reg_id >= arm64::Register::V0 as i32
                    && curr_reg_id <= arm64::Register::V31 as i32)
            {
                ArrayVec::from_array_len(zeros, 16)
            } else {
                return Err(uc_error::ARG);
            }
        } else {
            return Err(uc_error::ARCH);
        };

        let err = unsafe { ffi::uc_reg_read(self.inner().uc, curr_reg_id, value.as_mut_ptr() as _) };

        if err == uc_error::OK {
            Ok(value)
        } else {
            Err(err)
        }
    }

    /// Read a signed 32-bit value from a register.
    pub fn reg_read_i32<T: Into<i32>>(&self, regid: T) -> Result<i32, uc_error> {
        let mut value: i32 = 0;
        let err =
            unsafe { ffi::uc_reg_read(self.inner().uc, regid.into(), &mut value as *mut i32 as _) };
        if err == uc_error::OK {
            Ok(value)
        } else {
            Err(err)
        }
    }

    /// Add a code hook.
    pub fn add_code_hook<F: 'a>(
        &mut self,
        begin: u64,
        end: u64,
        callback: F,
    ) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u64, u32) + 'a,
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::CODE,
                ffi::code_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                begin,
                end,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));
            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add a block hook.
    pub fn add_block_hook<F: 'a>(&mut self, callback: F) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u64, u32),
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::BLOCK,
                ffi::block_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                1,
                0,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add a memory hook.
    pub fn add_mem_hook<F: 'a>(
        &mut self,
        hook_type: HookType,
        begin: u64,
        end: u64,
        callback: F,
    ) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, MemType, u64, usize, i64) -> bool,
    {
        if !(HookType::MEM_ALL | HookType::MEM_READ_AFTER).contains(hook_type) {
            return Err(uc_error::ARG);
        }

        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                hook_type,
                ffi::mem_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                begin,
                end,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add an interrupt hook.
    pub fn add_intr_hook<F: 'a>(&mut self, callback: F) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u32),
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::INTR,
                ffi::intr_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                0,
                0,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add hook for x86 IN instruction.
    pub fn add_insn_in_hook<F: 'a>(&mut self, callback: F) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u32, usize),
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::INSN,
                ffi::insn_in_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                0,
                0,
                x86::Insn::IN,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add hook for x86 OUT instruction.
    pub fn add_insn_out_hook<F: 'a>(&mut self, callback: F) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>, u32, usize, u32),
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::INSN,
                ffi::insn_out_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                0,
                0,
                x86::Insn::OUT,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Add hook for x86 SYSCALL or SYSENTER.
    pub fn add_insn_sys_hook<F>(
        &mut self,
        insn_type: x86::InsnSys,
        begin: u64,
        end: u64,
        callback: F,
    ) -> Result<ffi::uc_hook, uc_error>
    where
        F: FnMut(&mut Unicorn<D>) + 'a,
    {
        let mut hook_ptr = ptr::null_mut();
        let mut user_data = Box::new(ffi::UcHook {
            callback,
            uc: Unicorn {
                inner: self.inner.clone(),
            },
        });

        let err = unsafe {
            ffi::uc_hook_add(
                self.inner().uc,
                &mut hook_ptr,
                HookType::INSN,
                ffi::insn_sys_hook_proxy::<D, F> as _,
                user_data.as_mut() as *mut _ as _,
                begin,
                end,
                insn_type,
            )
        };
        if err == uc_error::OK {
            self.inner_mut().hooks.push((hook_ptr, user_data));

            Ok(hook_ptr)
        } else {
            Err(err)
        }
    }

    /// Remove a hook.
    ///
    /// `hook` is the value returned by `add_*_hook` functions.
    pub fn remove_hook(&mut self, hook: ffi::uc_hook) -> Result<(), uc_error> {
        let err: uc_error;

        // drop the hook
        self.inner_mut()
            .hooks
            .retain(|(hook_ptr, _hook_impl)| hook_ptr != &hook);

        err = unsafe { ffi::uc_hook_del(self.inner().uc, hook) };

        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Allocate and return an empty Unicorn context.
    ///
    /// To be populated via `context_save`.
    pub fn context_alloc(&self) -> Result<Context, uc_error> {
        let mut empty_context: ffi::uc_context = ptr::null_mut();
        let err = unsafe { ffi::uc_context_alloc(self.inner().uc, &mut empty_context) };
        if err == uc_error::OK {
            Ok(Context {
                context: empty_context,
            })
        } else {
            Err(err)
        }
    }

    /// Save current Unicorn context to previously allocated Context struct.
    pub fn context_save(&self, context: &mut Context) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_context_save(self.inner().uc, context.context) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Allocate and return a Context struct initialized with the current CPU context.
    ///
    /// This can be used for fast rollbacks with `context_restore`.
    /// In case of many non-concurrent context saves, use `context_alloc` and *_save
    /// individually to avoid unnecessary allocations.
    pub fn context_init(&self) -> Result<Context, uc_error> {
        let mut new_context: ffi::uc_context = ptr::null_mut();
        let err = unsafe { ffi::uc_context_alloc(self.inner().uc, &mut new_context) };
        if err != uc_error::OK {
            return Err(err);
        }
        let err = unsafe { ffi::uc_context_save(self.inner().uc, new_context) };
        if err == uc_error::OK {
            Ok(Context {
                context: new_context,
            })
        } else {
            unsafe { ffi::uc_context_free(new_context) };
            Err(err)
        }
    }

    /// Restore a previously saved Unicorn context.
    ///
    /// Perform a quick rollback of the CPU context, including registers and some
    /// internal metadata. Contexts may not be shared across engine instances with
    /// differing arches or modes. Memory has to be restored manually, if needed.
    pub fn context_restore(&self, context: &Context) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_context_restore(self.inner().uc, context.context) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Emulate machine code for a specified duration.
    ///
    /// `begin` is the address where to start the emulation. The emulation stops if `until`
    /// is hit. `timeout` specifies a duration in microseconds after which the emulation is
    /// stopped (infinite execution if set to 0). `count` is the maximum number of instructions
    /// to emulate (emulate all the available instructions if set to 0).
    pub fn emu_start(
        &mut self,
        begin: u64,
        until: u64,
        timeout: u64,
        count: usize,
    ) -> Result<(), uc_error> {
        unsafe {
            let err = ffi::uc_emu_start(self.inner().uc, begin, until, timeout, count as _);
            if err == uc_error::OK {
                Ok(())
            } else {
                Err(err)
            }
        }
    }

    /// Stop the emulation.
    ///
    /// This is usually called from callback function in hooks.
    /// NOTE: For now, this will stop the execution only after the current block.
    pub fn emu_stop(&mut self) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_emu_stop(self.inner().uc) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Query the internal status of the engine.
    ///
    /// supported: `MODE`, `PAGE_SIZE`, `ARCH`
    pub fn query(&self, query: Query) -> Result<usize, uc_error> {
        let mut result: libc::size_t = Default::default();
        let err = unsafe { ffi::uc_query(self.inner().uc, query, &mut result) };
        if err == uc_error::OK {
            Ok(result)
        } else {
            Err(err)
        }
    }

    /// Gets the current program counter for this `unicorn` instance.
    #[inline]
    pub fn pc_read(&self) -> Result<u64, uc_error> {
        let arch = self.get_arch();
        let reg = match arch {
            Arch::X86 => x86::Register::RIP as i32,
            Arch::ARM => arm::Register::PC as i32,
            Arch::ARM64 => arm64::Register::PC as i32,
            Arch::MIPS => mips::Register::PC as i32,
            Arch::SPARC => sparc::Register::PC as i32,
            Arch::M68K => m68k::Register::PC as i32,
            Arch::PPC => ppc::Register::PC as i32,
            Arch::RISCV => riscv::Register::PC as i32,
            Arch::MAX => panic!("Illegal Arch specified"),
        };
        self.reg_read(reg)
    }

    /// Sets the program counter for this `unicorn` instance.
    #[inline]
    pub fn pc_write(&mut self, value: u64) -> Result<(), uc_error> {
        let arch = self.get_arch();
        let reg = match arch {
            Arch::X86 => x86::Register::RIP as i32,
            Arch::ARM => arm::Register::PC as i32,
            Arch::ARM64 => arm64::Register::PC as i32,
            Arch::MIPS => mips::Register::PC as i32,
            Arch::SPARC => sparc::Register::PC as i32,
            Arch::M68K => m68k::Register::PC as i32,
            Arch::PPC => ppc::Register::PC as i32,
            Arch::RISCV => riscv::Register::PC as i32,
            Arch::MAX => panic!("Illegal Arch specified"),
        };
        self.reg_write(reg, value)
    }
}
