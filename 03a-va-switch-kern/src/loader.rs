#[cfg(not(test))]
global_asm!(include_str!("link_apps.S"));

use crate::mm;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::fmt;
use core::ops;

#[derive(Debug, Clone)]
pub struct AppLoader<'a> {
    apps: Vec<App<'a>>,
}

impl<'a> AppLoader<'a> {
    pub fn new() -> AppLoader<'a> {
        extern "C" { fn _app_meta(); }
        let num_app_ptr = _app_meta as usize as *const usize;
        let num_app = unsafe { num_app_ptr.read_volatile() };
        let mut apps = Vec::with_capacity(num_app);
        let mut cur = unsafe { num_app_ptr.offset(1) };
        for _ in 0..num_app {
            let name_len = unsafe { cur.read_volatile() };
            unsafe { cur = cur.offset(1) };
            let name_slice = unsafe { core::slice::from_raw_parts(cur as *const u8, name_len) };
            let name = alloc::str::from_utf8(name_slice).unwrap();
            unsafe { cur = (cur as *const u8).offset(name_len as isize) as *const usize };
            let start = unsafe { cur.read_volatile() }; 
            unsafe { cur = cur.offset(1) };
            let end = unsafe { cur.read_volatile() }; 
            unsafe { cur = cur.offset(1) };
            let elf_file = unsafe { core::slice::from_raw_parts(start as *const u8, end - start) };
            apps.push(App { name, elf_file });
        }
        AppLoader { apps }
    }
}

#[derive(Clone)]
struct App<'a> {
    name: &'a str,
    elf_file: &'a [u8],
}

impl fmt::Debug for App<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
         .field("name", &self.name)
         .field("elf_file", &(&self.elf_file.as_ptr(), &self.elf_file.len()))
         .finish()
    }
}

// todo: parse elf file
