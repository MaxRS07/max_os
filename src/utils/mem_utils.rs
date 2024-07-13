use crate::{println, vga::*};
use core::*;

pub unsafe fn search(start: u64, end: u64, lookup: &[u8], step: usize) -> Option<u64> {
    if start > end {
        return None;
    }
    for addr in (start..end - lookup.len() as u64).step_by(step) {
        let mut s = true;
        for i in 0..lookup.len() {
            if !s {
                continue;
            }
            let ptr: *const u8 = (addr + i as u64) as *const u8;
            if !(*ptr).eq(&lookup[i]) {
                s = false;
            }
        }
        if s {
            return Some(addr);
        }
    }
    None
}
pub unsafe fn object_search<T, P>(start: u64, end: u64, p: P) -> Option<u64>
where
    T: Sized,
    P: Fn(&T) -> bool,
{
    if start > end {
        return None;
    }
    for addr in (start..end - size_of::<T>() as u64).step_by(1) {
        let obj = &*(addr as *const T);
        if p(obj) {
            println!("{}", as_bytes(obj).as_str());
            //return Some(addr);
        }
    }
    None
}
pub fn bytes_from(start: u32, list: &mut [u8]) {
    unsafe {
        for i in 0..list.len() {
            let byte = (start + i as u32) as *const u8;
            list[i] = *byte;
        }
    }
}
pub fn verify_checksum<T>(object: &T) -> bool {
    unsafe {
        let mut bytes = core::slice::from_raw_parts(
            (object as *const _) as *const u8,
            core::mem::size_of::<T>(),
        );
        let mut checksum = 0x00;
        for byte in bytes {
            checksum ^= *byte;
        }
        checksum == 0x00
    }
}
pub fn create_checksum<T>(object: &T) -> u8 {
    unsafe {
        let mut bytes = core::slice::from_raw_parts(
            (object as *const _) as *const u8,
            core::mem::size_of::<T>(),
        );
        let mut checksum = 0x00;
        for byte in bytes {
            checksum ^= *byte;
        }
        0xFF - checksum
    }
}
pub fn addr<T>(object: &T) -> u64 {
    (object as *const _) as u64
}
pub fn as_bytes<T: Sized>(object: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts(object as *const T as *const u8, mem::size_of::<T>()) }
}
pub trait Strable {
    fn as_str(&self) -> &str;
}
impl Strable for [u8] {
    fn as_str(&self) -> &str {
        core::str::from_utf8(self).unwrap()
    }
}
