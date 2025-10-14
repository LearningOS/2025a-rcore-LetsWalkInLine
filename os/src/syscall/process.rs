//! Process management syscalls
use crate::mm::{PageTable, PhysAddr, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, get_syscalls_count, suspend_current_and_run_next
};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let user_pagetable = PageTable::from_token(current_user_token());
    let sec_va = VirtAddr::from(ts as usize);
    let usec_va = VirtAddr::from(ts as usize + core::mem::size_of::<usize>());
    //简单粗暴试一下
    let sec_ppn = user_pagetable
        .translate(sec_va.floor())
        .unwrap()
        .ppn();
    let usec_ppn = user_pagetable
        .translate(usec_va.floor())
        .unwrap()
        .ppn();
    let sec_pa = PhysAddr::from(sec_ppn).0 + sec_va.page_offset();
    let usec_pa = PhysAddr::from(usec_ppn).0 + usec_va.page_offset();
    let us = get_time_us();
    unsafe {
        *(sec_pa as *mut usize) = us / 1_000_000;
        *(usec_pa as *mut usize) = us % 1_000_000;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        0 => {
            let user_pagetable = PageTable::from_token(current_user_token());
            let va = VirtAddr::from(id);
            let pte = user_pagetable.translate(va.floor()).unwrap();
            if !(pte.is_valid() && pte.readable()) {
                return -1;
            }
            pte.ppn().get_bytes_array()[va.page_offset()] as isize
        },
        1 => {
            let user_pagetable = PageTable::from_token(current_user_token());
            let va = VirtAddr::from(id);
            let pte = user_pagetable.translate(va.floor()).unwrap();
            if !(pte.is_valid() && pte.writable()) {
                return -1;
            }
            pte.ppn().get_bytes_array()[va.page_offset()] = data as u8;

            0
        },
        2 => get_syscalls_count(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
