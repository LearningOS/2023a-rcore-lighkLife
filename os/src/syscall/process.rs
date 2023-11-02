//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
};
use crate::config::PAGE_SIZE;
use crate::mm::{MapPermission, VirtAddr};
use crate::timer::{get_time_us, MICRO_PER_MILLISECONDS, MICRO_PER_SEC};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / MICRO_PER_SEC,
        usec: us % MICRO_PER_SEC,
    };

    let virt_addr = VirtAddr::from(_ts as *const usize as usize);
    write(virt_addr, time_val);
    0
}

/// write value to the virtual address
fn write<T: Sized>(virt_addr: VirtAddr, value: T) {
    let task =  current_task().unwrap();
    let task = task.inner_exclusive_access();
    let ppn = task.memory_set.translate(virt_addr.floor()).unwrap();
    let page = ppn.ppn().get_bytes_array();

    let offset = virt_addr.page_offset();
    let bytes = any_as_u8_slice(&value);
    if offset + bytes.len() > PAGE_SIZE {
        //todo
        // find next page and write to it
        error!("split by two pages");
    }
    page[offset..offset + bytes.len()].copy_from_slice(&bytes);
}

/// convert any type value to [u8]
fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            (p as *const T) as *const u8,
            ::core::mem::size_of::<T>(),
        )
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task().unwrap().pid.0
    );
    let task =  current_task().unwrap();
    let task = task.inner_exclusive_access();
    let syscall_times = &mut [0; MAX_SYSCALL_NUM];
    syscall_times.copy_from_slice(task.get_syscall_times());
    let info = TaskInfo {
        status: task.get_status(),
        syscall_times: *syscall_times,
        time: task.get_time() / MICRO_PER_MILLISECONDS,
    };

    let addr = VirtAddr::from(_ti as *const usize as usize);
    write(addr, info);
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_addr = VirtAddr::from(_start);
    let end_addr = VirtAddr::from(_start + _len);
    // info!("kernel: sys_mmap {:?}, {:?}", start_addr, end_addr);
    if 0 != (start_addr.0 % PAGE_SIZE) {
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
    let perm = MapPermission::from_bits((_port << 1 | 1 << 4) as u8);
    if perm.is_none() {
        return -1;
    }
    let task =  current_task().unwrap();
    let mut task = task.inner_exclusive_access();
    task.memory_set.mmap(start_addr, end_addr, perm.unwrap())
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if 0 != (_start % PAGE_SIZE) {
        // start 没有按页大小对齐
        return -1;
    }
    let start_addr = VirtAddr::from(_start);
    let end_addr = VirtAddr::from(_start + _len);
    let task =  current_task().unwrap();
    let mut task = task.inner_exclusive_access();
    task.memory_set.unmmap(start_addr, end_addr)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(elf_data) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let current_task = current_task().unwrap();
        current_task.spawn(elf_data.read_all().as_slice())
    } else {
        //无效的文件名
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if _prio < 2 {
        return -1;
    }
    current_task().unwrap().change_priority(_prio as u32);
    _prio
}
