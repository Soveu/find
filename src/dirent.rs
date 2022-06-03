use crate::syscall;
use crate::directory::Directory;

use std::os::unix::io::AsRawFd;
use std::ffi::CStr;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum DirentType {
    Unknown = 0,
    Fifo = 1,
    Character = 2,
    Directory = 4,
    Block = 6,
    RegularFile = 8,
    Link = 10,
    Socket = 12,
    Wht = 14,
}

#[derive(Debug)]
#[repr(C)]
pub struct Dirent64 {
    pub inode: u64,
    pub offset: u64,
    pub reclen: u16,
    pub typ: u8,
    pub filename: CStr,
}

pub fn getdents64<'a>(
    fd: &Directory,
    dirp: &'a mut [u64]
) -> (DirentIter<'a>, &'a mut [u64]) {
    let fd = fd.0.as_raw_fd();
    let status = unsafe { syscall::getdents64(fd, dirp.as_mut_ptr(), dirp.len() * 8) };
    assert!(status > 0 && status % 8 == 0, "Error code = {}, fd = {}", -status, fd);
    let (dirents, rest) = dirp.split_at_mut(status as usize / 8);
    return (DirentIter(dirents), rest);
}

pub struct DirentIter<'a>(pub &'a [u64]);

impl<'a> Iterator for DirentIter<'a> {
    type Item = &'a Dirent64;

    fn next(&mut self) -> Option<Self::Item> {
        let base_dirent = self.0.get(..3)?;
        let base_dirent = std::ptr::slice_from_raw_parts(base_dirent.as_ptr(), 0);
        let base_dirent = base_dirent as *const Dirent64;
        let base_dirent = unsafe { &*base_dirent };

        let reclen = base_dirent.reclen as usize;
        let size_in_u64s = reclen / 8;
        let last_word_idx = size_in_u64s.checked_sub(1)?;
        assert!(reclen % 8 == 0 && reclen >= 24, "reclen={}", reclen);
        let last_word = self.0[last_word_idx];

        // Sometimes the filename will be so short, the last u64 will also contain
        // the contents of `reclen` and `type`, so we need to force them to
        // not have null bytes.
        let last_word = if last_word_idx == 2 { last_word | 0xFFFFFF } else { last_word };

        let ignore = 7 - last_word.to_le_bytes()
            .into_iter()
            .enumerate()
            .find(|(_, x)| *x == 0u8)
            .unwrap()
            .0;

        let dirent_size = 8 + 8 + 2 + 1;
        let filename_len = reclen - dirent_size - ignore;
        let (dirent, rest) = self.0.split_at(size_in_u64s);
        let dirent = std::ptr::slice_from_raw_parts(dirent.as_ptr(), filename_len);
        let dirent = dirent as *const Dirent64;
        let dirent = unsafe { &*dirent };

        self.0 = rest;
        return Some(dirent);
    }
}
