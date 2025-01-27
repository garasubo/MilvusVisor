# MilvusVisor
MilvusVisor is a thin hypervisor that runs on aarch64 CPUs.

The features of the MilvusVisor are the following.

- (Theoretically) Smaller footprint and overhead than typical VMM (e.g. QEMU/KVM, Xen)
- Support running only one guest OS simultaneously for keeping it simple and thin
- Written in Rust


MilvusVisor allows providing functions OS-independently without the overhead of device virtualization.
We are currently developing MilvusVisor as a research activity to achieve HPC environments that provide root privilege to the users without virtualization overhead.

## Functions

Currently, MilvusVisor provides the following function.

You can build with enabling some functions by `make custom_all FEATURES=feature1,feautre2,...`.(`featureN` is described like `Feature Name: feature_name` in each section.)

- Protecting non-volatile data in devices from guest OS (e.g. Firmware, MAC address)
  - Intel I210 (Feature Name: `i210`)
    - Protect EEPROM from writing access
  - Mellanox Technologies MT27800 (Feature Name: `mt27800`)
    - Protect from firmware update
- Protecting MilvusVisor itself against DMA attack (Feature Name: `smmu`)
  - Using SMMUv3 Stage 2 Page Translation to protect from DMA attack
  - Stage 1 translation is available from guest OS
- Fast restore: Fast restoring the guest environments without reboot the machine (Feature Name: `fast_restore`)
  - Taking a snapshot just before the first boot of the guest OS
  - Restoring it on rebooting/shutting down the guest OS
- Protecting ACPI Tables from write accesses (Feature Name: `acpi_table_protection`)
  - For the Fast Restore
- Linked-List Style Memory Allocator (Feature Name:  `advanced_memory_manager`)
- Contiguous Bit (Feature Name: `contiguous_bit`)
  - Set contiguous bit enabled  if available (TLB will be optimized by the contiguous bit)
  - Some machine may not work fine with the contiguous bit
- A64FX specific registers' initialization (Feature Name: `a64fx`)
  - Initialize some A64FX specific registers during boot
- PXE Boot (Feature Name: `tftp`)
  - download hypervisor_kernel and payload(usually, bootloader) via TFTP

## Tested machines

We have tested MilvusVisor on the following machines.

- FUJITSU FX700
- GIGABYTE E252-P30
- QEMU

The following table shows which feature worked on which machines.

| Test items \\ Machine                                       | FX700 | E252-P30 | QEMU |
|:------------------------------------------------------------|:-----:|:--------:|:----:|
| Booting Linux on MilvusVisor (Multi-core)                   | o     | o        | o   |
| Protecting non-volatile data of Intel I210                  | o     | -        | -   |
| Protecting firmware update of Mellanox Technologies MT27800 | o     | -        | -   |
| Protecting MilvusVisor itself against DMA attack            | o     | -        | -   |
| Fast Restore                                                | o     | -        | -   |

## How to build the hypervisor

### By Rust toolchain

#### Requirements
- `rustup` command-line tool (you can install from https://rustup.rs/)

#### Steps (commands list)
```bash
rustup component add rust-src
cd path/to/repo-root/src
make
```

If you want to enable/disable specific features, please execute `make custom_all FEATURES=(comma separated features)`.
Please see `src/hypervisor_kernel/Cargo.toml` to know what features are available.

For example, if you want to build the hypervisor only with the device protection features, you should run the below command instead of `make`

```bash
make custom_all FEATURES=i210,mt27800
```

Next [How to run the hypervisor](#how-to-run-the-hypervisor)

### By docker
#### Requirements
- Docker (Tested by `Docker version 20.10.8, build 3967b7d28e`)
  - I tested by non-root users (See [this](https://docs.docker.com/engine/install/linux-postinstall/#manage-docker-as-a-non-root-user) to run docker command by non-root user)

#### Steps (commands list)

```bash
cd path/to/repo-root/src
./build_by_docker.sh # You can add arguments to pass the make command, like as `custom_all FEATURES=...`
```
For more detail, please see the scripts.


## How to run the hypervisor
### On QEMU
First, please install QEMU that supports emulating `QEMU ARM Virtual Machine`, `a64fx` CPU.
Then, run the following command to run the built hypervisor.

```bash
cd path/to/repo-root/src
make QEMU_EFI=/usr/share/qemu-efi/QEMU_EFI.fd run #Please set the path of your QEMU_EFI.fd to QEMU_EFI
```

### On a physical machine from a USB memory stick
#### Requirement
- Prepare a USB memory that has an EFI (FAT) partition that has `/EFI/BOOT/` directory. Please confirm that there is no important file in the partition.
- Prepare a physical machine that has ARMv8.1-A or later, and UEFI firmware.

#### Steps
1. Attach your USB memory stick to the development machine which built the hypervisor binary.
2. Identify the EFI partition (in the following description, `/dev/sdX1` is the EFI partition).
3. Run `sudo make DEVICE=/dev/sdX1 write` to copy the binary.
   !! Please be careful not to specify a wrong partition as `DEVICE` because the script mount/unmount the partition and copies the binary file with root privilege.!!
4. Detach the USB memory from the development machine, and attach it to the physical machine to run the hypervisor.
5. Boot the physical machine with UEFI, and specify `BOOTAA64.EFI` in the EFI partition as the EFI application to boot.

## How to generate the documentation
You can generate the document by `cargo doc` in each cargo project directory.

If you want to see bootloader's document, please run the following command.

```bash
cd path/to/repo-root/src/hypervisor_bootloader
cargo doc --open # Browser will open
```

If you want to see kernel's document, please run the following command.

```bash
cd path/to/repo-root/src/hypervisor_kernel
cargo doc --open # Browser will open
```

## Acknowledgment
This work was supported by JSPS KAKENHI Grant Number 21K17727.
