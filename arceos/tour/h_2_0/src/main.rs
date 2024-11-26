#![no_std]
#![no_main]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;
extern crate axstd as std;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};
use riscv_vcpu::AxVCpuExitReason;
use axerrno::{ax_err, ax_err_type, AxResult};
use memory_addr::VirtAddr;
use alloc::string::String;
use std::fs::File;
use riscv_vcpu::RISCVVCpu;
use riscv_vcpu::AxVCpuExitReason::NestedPageFault;

const VM_ASPACE_BASE: usize = 0x0;
const VM_ASPACE_SIZE: usize = 0x7fff_ffff_f000;

use axmm::AddrSpace;
use axhal::paging::MappingFlags;

#[no_mangle]
fn main() {
    info!("Starting virtualization...");
    unsafe {
        riscv_vcpu::setup_csrs();
    }

    // Set up Memory regions.
    let mut aspace = AddrSpace::new_empty(VirtAddr::from(VM_ASPACE_BASE), VM_ASPACE_SIZE).unwrap();

    let gpa = 0x8000_0000;
    let g_size = 0x100_0000;
    let mapping_flags = MappingFlags::from_bits(0xf).unwrap();
    aspace.map_alloc(
        gpa.into(),
        g_size,
        mapping_flags,
        true,
    ).unwrap();

    // Load corresponding images for VM.
    info!("VM created success, loading images...");
    let gpa = 0x8020_0000;
    //let image_fname = "starry-next_riscv64-qemu-virt.bin";
    //let image_fname = "arceos-riscv64.bin";
    let image_fname = "/sbin/u_3_0_riscv64-qemu-virt.bin";
    load_vm_image(image_fname.to_string(), gpa.into(), &aspace).expect("Failed to load VM images");

    // Create VCpus.
    let mut arch_vcpu = RISCVVCpu::init();

    // Setup VCpus.
    let entry = 0x8020_0000;
    info!("bsp_entry: {:#x}; ept: {:#x}", entry, aspace.page_table_root());
    arch_vcpu.set_entry(entry.into()).unwrap();
    arch_vcpu.set_ept_root(aspace.page_table_root()).unwrap();

    loop {
        match vcpu_run(&mut arch_vcpu) {
            Ok(exit_reason) => match exit_reason {
                AxVCpuExitReason::Nothing => {},
                NestedPageFault{addr, access_flags} => {
                    info!("addr {:#x} access {:#x}", addr, access_flags);
                    let mapping_flags = MappingFlags::from_bits(0xf).unwrap();
                    aspace.map_alloc(addr, 4096, mapping_flags, true);
                    let buf = "pfld";
                    aspace.write(addr, buf.as_bytes());
                    //aspace.read(addr, &mut buf);
                    //error!("buf: {:?}", buf);
                },
                _ => {
                    panic!("Unhandled VM-Exit: {:?}", exit_reason);
                }
            },
            Err(err) => {
                panic!("run VCpu get error {:?}", err);
            }
        }
    }

    unreachable!("VMM start failed")
}

fn load_vm_image(image_path: String, image_load_gpa: VirtAddr, aspace: &AddrSpace) -> AxResult {
    error!("*********** load_vm_image: {} {:?}", image_path, image_load_gpa);
    use std::io::{BufReader, Read};
    let (image_file, image_size) = open_image_file(image_path.as_str())?;

    let image_load_regions = aspace
        .translated_byte_buffer(image_load_gpa, image_size)
        .expect("Failed to translate kernel image load address");
    let mut file = BufReader::new(image_file);

    for buffer in image_load_regions {
        error!("*** buffer {}", buffer.len());
        file.read_exact(buffer).map_err(|err| {
            ax_err_type!(
                Io,
                format!("Failed in reading from file {}, err {:?}", image_path, err)
            )
        })?
    }

    Ok(())
}

fn vcpu_run(arch_vcpu: &mut RISCVVCpu) -> AxResult<AxVCpuExitReason> {
    use axhal::arch::{local_irq_save_and_disable, local_irq_restore};
    let flags = local_irq_save_and_disable();
    let ret = arch_vcpu.run();
    local_irq_restore(flags);
    ret
}

fn open_image_file(file_name: &str) -> AxResult<(File, usize)> {
    let file = File::open(file_name).map_err(|err| {
        ax_err_type!(
            NotFound,
            format!(
                "Failed to open {}, err {:?}, please check your disk.img",
                file_name, err
            )
        )
    })?;
    let file_size = file
        .metadata()
        .map_err(|err| {
            ax_err_type!(
                Io,
                format!(
                    "Failed to get metadate of file {}, err {:?}",
                    file_name, err
                )
            )
        })?
        .size() as usize;
    Ok((file, file_size))
}
