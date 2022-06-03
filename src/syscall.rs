use std::arch::asm;

#[allow(dead_code)]
pub unsafe fn close(fd: i32) {
    let output: isize;
    asm!("
        push rcx
        push r11
        syscall
        push r11
        popf
        pop r11
        pop rcx",
        inout("rax") 3usize => output,
        in("rdi") fd,
        options(preserves_flags, nomem)
    );
    assert_eq!(output, 0);
}

pub unsafe fn open(path: *const i8, flags: usize, mode: usize) -> isize {
    let output;
    asm!("
        push rcx
        push r11
        syscall
        push r11
        popf
        pop r11
        pop rcx",
        inout("rax") 2usize => output,
        in("rdi") path,
        in("rsi") flags,
        in("rdx") mode,
        options(preserves_flags, readonly)
    );
    return output;
}

pub unsafe fn getdents64(fd: i32, dirp: *mut u64, count: usize) -> isize {
    let output;
    asm!("
        push rcx
        push r11
        syscall
        push r11
        popf
        pop r11
        pop rcx",
        inout("rax") 217usize => output,
        in("rdi") fd,
        in("rsi") dirp,
        in("rdx") count,
        options(preserves_flags)
    );
    return output;
}
