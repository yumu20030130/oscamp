#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(riscv_ext_intrinsics)]

#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;
#[macro_use]
extern crate axlog;

mod task;
mod vcpu;
mod regs;
mod csrs;
mod sbi;

use std::io::{self, Read};
use std::fs::File;
use axhal::paging::MappingFlags;
use axhal::mem::{PAGE_SIZE_4K, phys_to_virt};
use vcpu::VmCpuRegisters;
use riscv::register::{scause, sstatus};
use csrs::defs::hstatus;
use tock_registers::LocalRegisterCopy;
use csrs::{RiscvCsrTrait, CSR};
use vcpu::_run_guest;
use sbi::SbiMessage;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    ax_println!("Hypervisor ...");

    let mut buf = [0u8; 64];
    if let Err(e) = load_user_app("/sbin/skernel", &mut buf) {
        panic!("Cannot load app! {:?}", e);
    }

    let entry = 0x8020_0000;
    let mut uspace = axmm::new_user_aspace().unwrap();
    uspace.map_alloc(entry.into(), PAGE_SIZE_4K, MappingFlags::READ|MappingFlags::WRITE|MappingFlags::EXECUTE|MappingFlags::USER, true).unwrap();

    let (paddr, _, _) = uspace
        .page_table()
        .query(entry.into())
        .unwrap_or_else(|_| panic!("Mapping failed for segment: {:#x}", entry));

    ax_println!("paddr: {:#x}", paddr);

    unsafe {
        core::ptr::copy_nonoverlapping(
            buf.as_ptr(),
            phys_to_virt(paddr).as_mut_ptr(),
            PAGE_SIZE_4K,
        );
    }

    ax_println!("New user address space: {:#x?}", uspace);

    let ept_root = uspace.page_table_root();
    let mut ctx = VmCpuRegisters::default();
    // Set hstatus
    let mut hstatus = LocalRegisterCopy::<usize, hstatus::Register>::new(
        riscv::register::hstatus::read().bits(),
    );
    hstatus.modify(hstatus::spv::Guest);
    // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // Set sstatus
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    ctx.guest_regs.sstatus = sstatus.bits();

    ctx.guest_regs.sepc = entry;
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();
    }

    run_guest(&mut ctx);

    panic!("Hypervisor ok!");
}

fn run_guest(ctx: &mut VmCpuRegisters) {
    unsafe {
        _run_guest(ctx);
    }

    vmexit_handler(ctx)
}

fn vmexit_handler(ctx: &VmCpuRegisters) {
    use scause::{Exception, Trap};

    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            if let Some(msg) = sbi_msg {
                match msg {
                    SbiMessage::Reset(_) => {
                        ax_println!("Shutdown vm normally!");
                        sbi_rt::system_reset(sbi_rt::Shutdown, sbi_rt::SystemFailure);
                    },
                    _ => todo!(),
                }
            } else {
                panic!("bad sbi message! ");
            }
        },
        _ => {
            panic!(
                "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                scause.cause(),
                ctx.guest_regs.sepc,
                ctx.trap_csrs.stval
            );
        }
    }
}

fn load_user_app(fname: &str, buf: &mut [u8]) -> io::Result<usize> {
    ax_println!("app: {}", fname);
    let mut file = File::open(fname)?;
    let n = file.read(buf)?;
    Ok(n)
}
