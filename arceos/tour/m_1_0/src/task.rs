use alloc::sync::Arc;

use axhal::arch::UspaceContext;
use axmm::AddrSpace;
use axsync::Mutex;
use axtask::{AxTaskRef, TaskExtRef, TaskInner};

/// Task extended data for the monolithic kernel.
pub struct TaskExt {
    /// The process ID.
    pub proc_id: usize,
    /// The user space context.
    pub uctx: UspaceContext,
    /// The virtual memory address space.
    pub aspace: Arc<Mutex<AddrSpace>>,
}

impl TaskExt {
    pub const fn new(uctx: UspaceContext, aspace: Arc<Mutex<AddrSpace>>) -> Self {
        Self {
            proc_id: 1,
            uctx,
            aspace,
        }
    }
}

axtask::def_task_ext!(TaskExt);

pub fn spawn_user_task(aspace: Arc<Mutex<AddrSpace>>, uctx: UspaceContext) -> AxTaskRef {
    let mut task = TaskInner::new(
        // 传入该进程的入口函数
        || {
            let curr = axtask::current();
            let kstack_top = curr.kernel_stack_top().unwrap();
            ax_println!(
                "Enter user space: entry={:#x}, ustack={:#x}, kstack={:#x}",
                curr.task_ext().uctx.get_ip(),
                curr.task_ext().uctx.get_sp(),
                kstack_top,
            );
            unsafe { curr.task_ext().uctx.enter_uspace(kstack_top) };
        },
        "userboot".into(),
        crate::KERNEL_STACK_SIZE,
    );
    // 设置宏内核特有的资源
    // 设置该进程的用户页表（但还没启用）
    task.ctx_mut()
        .set_page_table_root(aspace.lock().page_table_root());
    // 设置进程的拓展属性（因宏内核模式而产生）
    task.init_task_ext(TaskExt::new(uctx, aspace));

    // 正常调用原来的spawn函数
    axtask::spawn_task(task)
}
