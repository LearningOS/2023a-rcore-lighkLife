//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    },
};
use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, VirtAddr};
use crate::task::{alloc, current_task_status, current_task_sys_call_times, current_task_time, translate};
use crate::timer::get_time_us;

const MICRO_PER_SEC: usize = 1_000_000;
const US_PER_MS: usize = 1_000;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Debug)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / MICRO_PER_SEC,
        usec: us % MICRO_PER_SEC,
    };

    let virt_addr = VirtAddr::from(_ts as *const usize as usize);
    write(virt_addr, time_val);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");

    let syscall_times = &mut [0; MAX_SYSCALL_NUM];
    current_task_sys_call_times(syscall_times);
    let info = TaskInfo {
        status: current_task_status(),
        syscall_times: *syscall_times,
        time: current_task_time() / US_PER_MS,
    };

    let virt_addr = VirtAddr::from(_ti as *const usize as usize);
    write(virt_addr, info);
    0
}

fn write<T: Sized>(virt_addr: VirtAddr, value: T) {
    let ppn = translate(virt_addr.floor()).unwrap();
    let page = ppn.get_bytes_array();

    let offset = virt_addr.page_offset();
    let bytes = any_as_u8_slice(&value);
    if offset + bytes.len() > PAGE_SIZE {
        //todo
        // find next page and write to it
        error!("split by two pages");
    }
    page[offset..offset + bytes.len()].copy_from_slice(&bytes);
}

fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (p as *const T) as *const u8,
            ::core::mem::size_of::<T>(),
        )
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_addr = VirtAddr::from(_start);
    let end_addr = VirtAddr::from(_start + _len);
    if start_addr.page_offset() != 0 {
        // start 没有按页大小对齐
        return -1;
    }
    if _port & !0x7 != 0 {
        //port 其余位必须为0
        return -1;
    }
    if _port & 0x7 == 0 {
        //不能读、写、执行，这样的内存无意义
        return -1;
    }
    let perm = MapPermission::from_bits((_port << 1) as u8);
    if perm.is_none() {
        return -1;
    }
    alloc(start_addr, end_addr, perm.unwrap())
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
