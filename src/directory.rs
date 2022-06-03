use crate::syscall;

use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::os::unix::io::FromRawFd;

#[repr(transparent)]
pub struct Directory(pub File);

pub fn opendir(path: &CStr) -> io::Result<Directory> {
    const O_DIRECTORY: usize = 0o200_000;
    const O_NOFOLLOW: usize = 0o400_000;
    const O_RDONLY: usize = 0;
    const OPEN_FLAGS: usize = O_DIRECTORY | O_NOFOLLOW | O_RDONLY;

    let fd = unsafe { syscall::open(path.as_ptr(), OPEN_FLAGS, 0) };
    if fd < 0 {
        return Err(io::Error::from_raw_os_error(-fd as i32));
    }

    let f = unsafe { File::from_raw_fd(fd as i32) };
    return Ok(Directory(f));
}
