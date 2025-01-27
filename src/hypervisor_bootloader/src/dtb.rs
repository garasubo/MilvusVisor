// Copyright (c) 2022 RIKEN
// All rights reserved.
//
// This software is released under the MIT License.
// http://opensource.org/licenses/mit-license.php

const FDT_BEGIN_NODE: u32 = 0x00000001u32.to_be();
const FDT_END_NODE: u32 = 0x00000002u32.to_be();
const FDT_PROP: u32 = 0x00000003u32.to_be();
const FDT_NOP: u32 = 0x00000004u32.to_be();
const FDT_END: u32 = 0x00000009u32.to_be();
const TOKEN_SIZE: usize = 4;

//const NODE_NAME_SERIAL: &[u8] = "serial".as_bytes();

const PROP_STATUS: &[u8] = "status".as_bytes();
const PROP_STATUS_OKAY: &[u8] = "okay".as_bytes();
const PROP_COMPATIBLE: &[u8] = "compatible".as_bytes();
const PROP_ADDRESS_CELLS: &[u8] = "#address-cells".as_bytes();
const PROP_SIZE_CELLS: &[u8] = "#size-cells".as_bytes();
const PROP_REG: &[u8] = "reg".as_bytes();

const DEFAULT_ADDRESS_CELLS: u32 = 2;
const DEFAULT_SIZE_CELLS: u32 = 1;

#[repr(C)]
struct DtbHeader {
    magic: u32,
    total_size: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsv_map: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

impl DtbHeader {
    const DTB_MAGIC: u32 = 0xd00dfeedu32.to_be();
    pub fn check_magic(&self) -> bool {
        self.magic == Self::DTB_MAGIC
    }
}

#[derive(Clone)]
pub struct DtbNode {
    address_offset: usize,
    address_cells: u32,
    size_cells: u32,
    base_pointer: usize,
}

#[derive(Clone)]
pub struct DtbNodeNameSearchHolder {
    node: DtbNode,
    pointer: usize,
}

#[allow(dead_code)]
pub struct DtbAnalyser {
    struct_block_address: usize,
    struct_block_size: usize,
    strings_block_address: usize,
    strings_block_size: usize,
}

impl DtbNode {
    fn skip_nop(pointer: &mut usize) {
        while unsafe { *(*pointer as *const u32) } == FDT_NOP {
            *pointer += TOKEN_SIZE;
        }
        return;
    }

    fn skip_padding(pointer: &mut usize) -> Result<(), ()> {
        while (*pointer & (TOKEN_SIZE - 1)) != 0 {
            if unsafe { *(*pointer as *const u8) } != 0 {
                println!("Warning: Expected zero paddings, but found {:#X}", unsafe {
                    *(*pointer as *const u8)
                });
            }
            *pointer += 1;
        }
        return Ok(());
    }

    #[allow(dead_code)]
    fn match_name(mut pointer: usize, s: &[u8]) -> bool {
        for c in s {
            if unsafe { &*(pointer as *const u8) } != c {
                return false;
            }
            pointer += 1;
        }
        let last = unsafe { *(pointer as *const u8) };
        last == 0 || last == b'@'
    }

    fn match_string(mut pointer: usize, s: &[u8]) -> bool {
        for c in s {
            if unsafe { &*(pointer as *const u8) } != c {
                return false;
            }
            pointer += 1;
        }
        unsafe { *(pointer as *const u8) == 0 }
    }

    fn skip_to_end_of_node(pointer: &mut usize) -> Result<(), ()> {
        Self::skip_nop(pointer);
        if unsafe { *(*pointer as *const u32) } != FDT_BEGIN_NODE {
            println!(
                "Expected FDT_BEGIN_NODE, but found {:#X}",
                u32::from_be(unsafe { *(*pointer as *const u32) })
            );
            return Err(());
        }
        *pointer += TOKEN_SIZE;
        while unsafe { *(*pointer as *const u8) } != 0 {
            *pointer += 1;
        }
        *pointer += 1;
        Self::skip_padding(pointer)?;

        while unsafe { *(*pointer as *const u32) } != FDT_END_NODE {
            assert_eq!(*pointer & (TOKEN_SIZE - 1), 0);
            match unsafe { *(*pointer as *const u32) } {
                FDT_PROP => {
                    *pointer += TOKEN_SIZE;
                    let property_len = u32::from_be(unsafe { *(*pointer as *const u32) });
                    *pointer += core::mem::size_of::<u32>() * 2;
                    *pointer += property_len as usize;
                    Self::skip_padding(pointer)?;
                }
                FDT_BEGIN_NODE => {
                    Self::skip_to_end_of_node(pointer)?;
                }
                FDT_NOP => {}
                _ => {
                    println!(
                        "Expected TOKEN, but found {:#X}(Address: {:#X})",
                        u32::from_be(unsafe { *(*pointer as *const u32) }),
                        *pointer
                    );
                    return Err(());
                }
            }
            Self::skip_nop(pointer);
        }
        *pointer += TOKEN_SIZE;
        return Ok(());
    }

    fn search_pointer_to_property(
        &mut self,
        target_prop_name: &[u8],
        dtb: &DtbAnalyser,
    ) -> Result<Option<usize>, ()> {
        let mut pointer = self.base_pointer;
        Self::skip_nop(&mut pointer);
        if unsafe { *(pointer as *const u32) } != FDT_BEGIN_NODE {
            println!(
                "Expected FDT_BEGIN_NODE, but found {:#X}",
                u32::from_be(unsafe { *(pointer as *const u32) })
            );
            return Err(());
        }
        pointer += TOKEN_SIZE;
        while unsafe { *(pointer as *const u8) } != 0 {
            pointer += 1;
        }
        pointer += 1;
        Self::skip_padding(&mut pointer)?;

        while unsafe { *(pointer as *const u32) } != FDT_END_NODE {
            assert_eq!(pointer & (TOKEN_SIZE - 1), 0);
            match unsafe { *(pointer as *const u32) } {
                FDT_PROP => {
                    pointer += TOKEN_SIZE;
                    let property_len = u32::from_be(unsafe { *(pointer as *const u32) });
                    pointer += core::mem::size_of::<u32>();
                    let name_segment_offset = u32::from_be(unsafe { *((pointer) as *const u32) });
                    pointer += core::mem::size_of::<u32>();

                    let prop_name = dtb.get_name(name_segment_offset)?;
                    if Self::match_string(prop_name, PROP_ADDRESS_CELLS) {
                        self.address_cells = u32::from_be(unsafe { *(pointer as *const u32) });
                    } else if Self::match_string(prop_name, PROP_SIZE_CELLS) {
                        self.size_cells = u32::from_be(unsafe { *(pointer as *const u32) });
                    } else if Self::match_string(prop_name, target_prop_name) {
                        return Ok(Some(pointer));
                    }

                    pointer += property_len as usize;
                    Self::skip_padding(&mut pointer)?;
                }
                FDT_BEGIN_NODE => {
                    Self::skip_to_end_of_node(&mut pointer)?;
                }
                FDT_NOP => {}
                _ => {
                    println!(
                        "Expected TOKEN, but found {:#X}(Offset from current node: {:#X})",
                        u32::from_be(unsafe { *(pointer as *const u32) }),
                        pointer - self.base_pointer
                    );
                    return Err(());
                }
            }
            Self::skip_nop(&mut pointer);
        }
        return Ok(None);
    }

    fn _search_device_by_node_name(
        &mut self,
        node_name: &[u8],
        dtb: &DtbAnalyser,
        pointer: &mut usize,
    ) -> Result<Option<Self>, ()> {
        Self::skip_nop(pointer);
        if unsafe { *(*pointer as *const u32) } != FDT_BEGIN_NODE {
            println!(
                "Expected FDT_BEGIN_NODE, but found {:#X}",
                u32::from_be(unsafe { *(*pointer as *const u32) })
            );
            return Err(());
        }
        *pointer += TOKEN_SIZE;

        // TODO: delete println!("NodeName") and improve code to do both of matching the name and
        //       skipping left words if the name is not matched.
        let name_base = *pointer;
        while unsafe { *(*pointer as *const u8) } != 0 {
            *pointer += 1;
        }
        let name_len = *pointer - name_base;

        println!(
            "NodeName: {}",
            unsafe {
                core::str::from_utf8(core::slice::from_raw_parts(
                    name_base as *const u8,
                    name_len,
                ))
            }
            .unwrap_or("???")
        );
        let is_name_matched = Self::match_name(name_base, node_name);

        *pointer += 1;

        Self::skip_padding(pointer)?;

        self.__search_device_by_node_name(node_name, dtb, pointer, is_name_matched)
    }

    fn __search_device_by_node_name(
        &mut self,
        node_name: &[u8],
        dtb: &DtbAnalyser,
        pointer: &mut usize,
        is_name_matched: bool,
    ) -> Result<Option<Self>, ()> {
        while unsafe { *(*pointer as *const u32) } != FDT_END_NODE {
            assert_eq!(*pointer & (TOKEN_SIZE - 1), 0);
            match unsafe { *(*pointer as *const u32) } {
                FDT_PROP => {
                    *pointer += TOKEN_SIZE;
                    let property_len = u32::from_be(unsafe { *(*pointer as *const u32) });
                    *pointer += core::mem::size_of::<u32>();
                    let name_segment_offset = u32::from_be(unsafe { *((*pointer) as *const u32) });
                    *pointer += core::mem::size_of::<u32>();

                    let prop_name = dtb.get_name(name_segment_offset)?;
                    if Self::match_string(prop_name, PROP_ADDRESS_CELLS) {
                        self.address_cells = u32::from_be(unsafe { *(*pointer as *const u32) });
                    } else if Self::match_string(prop_name, PROP_SIZE_CELLS) {
                        self.size_cells = u32::from_be(unsafe { *(*pointer as *const u32) });
                    } else if Self::match_string(prop_name, PROP_REG) {
                        println!(
                            "Reg: {:#X} {:#X}, Size: {:#X}",
                            u32::from_be(unsafe { *(*pointer as *const u32) }),
                            u32::from_be(unsafe { *((*pointer + 4) as *const u32) }),
                            self.address_cells
                        );
                        let mut p = *pointer;
                        let mut address_cells = 0usize;
                        for _ in 0..self.address_cells {
                            address_cells <<= u32::BITS;
                            address_cells |= u32::from_be(unsafe { *(p as *const u32) }) as usize;
                            p += TOKEN_SIZE;
                        }
                        self.address_offset += address_cells as usize;
                        println!("AddressOffset: {:#X}", self.address_offset);
                        if property_len
                            != (self.address_cells + self.size_cells) * TOKEN_SIZE as u32
                        {
                            println!(
                                "Expected {} bytes for reg, but found {} bytes",
                                (self.address_cells + self.size_cells) * TOKEN_SIZE as u32,
                                property_len
                            );
                            return Err(());
                        }
                    }

                    *pointer += property_len as usize;
                    Self::skip_padding(pointer)?;
                }
                FDT_BEGIN_NODE => {
                    if is_name_matched {
                        return Ok(Some(self.clone()));
                    }
                    let mut child = self.clone();
                    child.base_pointer = *pointer;
                    let result = child._search_device_by_node_name(node_name, dtb, pointer)?;
                    if result.is_some() {
                        return Ok(result);
                    }
                }
                FDT_NOP => {}
                _ => {
                    println!(
                        "Expected TOKEN, but found {:#X}(Offset from current node: {:#X})",
                        u32::from_be(unsafe { *(*pointer as *const u32) }),
                        *pointer - self.base_pointer
                    );
                    return Err(());
                }
            }
            Self::skip_nop(pointer);
        }
        if is_name_matched {
            return Ok(Some(self.clone()));
        }
        *pointer += TOKEN_SIZE;
        return Ok(None);
    }

    fn _search_device_by_compatible(
        &mut self,
        compatible_devices: &[&[u8]],
        dtb: &DtbAnalyser,
        pointer: &mut usize,
    ) -> Result<Option<(Self, usize)>, ()> {
        Self::skip_nop(pointer);

        if unsafe { *(*pointer as *const u32) } != FDT_BEGIN_NODE {
            println!(
                "Expected FDT_BEGIN_NODE, but found {:#X}",
                u32::from_be(unsafe { *(*pointer as *const u32) })
            );
            return Err(());
        }
        *pointer += TOKEN_SIZE;

        while unsafe { *(*pointer as *const u8) } != 0 {
            *pointer += 1;
        }
        *pointer += 1;

        Self::skip_padding(pointer)?;

        self.__search_device_by_compatible(compatible_devices, dtb, pointer)
    }

    fn __search_device_by_compatible(
        &mut self,
        compatible_devices: &[&[u8]],
        dtb: &DtbAnalyser,
        pointer: &mut usize,
    ) -> Result<Option<(Self, usize)>, ()> {
        let mut compatible_index: Option<usize> = None;

        while unsafe { *(*pointer as *const u32) } != FDT_END_NODE {
            assert_eq!(*pointer & (TOKEN_SIZE - 1), 0);
            match unsafe { *(*pointer as *const u32) } {
                FDT_PROP => {
                    *pointer += TOKEN_SIZE;
                    let property_len = u32::from_be(unsafe { *(*pointer as *const u32) });
                    *pointer += core::mem::size_of::<u32>();
                    let name_segment_offset = u32::from_be(unsafe { *((*pointer) as *const u32) });
                    *pointer += core::mem::size_of::<u32>();

                    let prop_name = dtb.get_name(name_segment_offset)?;
                    if Self::match_string(prop_name, PROP_COMPATIBLE) {
                        let mut list_pointer = 0usize;
                        'list_loop: while list_pointer < property_len as usize {
                            for (index, c_d) in compatible_devices.iter().enumerate() {
                                if Self::match_string(*pointer + list_pointer, c_d) {
                                    compatible_index = Some(index);
                                    break 'list_loop;
                                }
                            }
                            while unsafe { *((*pointer + list_pointer) as *const u8) } != 0 {
                                list_pointer += 1;
                            }
                            list_pointer += 1;
                        }
                    } else if Self::match_string(prop_name, PROP_ADDRESS_CELLS) {
                        self.address_cells = u32::from_be(unsafe { *(*pointer as *const u32) });
                    } else if Self::match_string(prop_name, PROP_SIZE_CELLS) {
                        self.size_cells = u32::from_be(unsafe { *(*pointer as *const u32) });
                    } else if Self::match_string(prop_name, PROP_REG) {
                        println!(
                            "Reg: {:#X} {:#X}, Size: {:#X}",
                            u32::from_be(unsafe { *(*pointer as *const u32) }),
                            u32::from_be(unsafe { *((*pointer + 4) as *const u32) }),
                            self.address_cells
                        );
                        let mut p = *pointer;
                        let mut address_cells = 0usize;
                        for _ in 0..self.address_cells {
                            address_cells <<= u32::BITS;
                            address_cells |= u32::from_be(unsafe { *(p as *const u32) }) as usize;
                            p += TOKEN_SIZE;
                        }
                        self.address_offset += address_cells as usize;
                        println!("AddressOffset: {:#X}", self.address_offset);
                        if property_len
                            != (self.address_cells + self.size_cells) * TOKEN_SIZE as u32
                        {
                            println!(
                                "Expected {} bytes for reg, but found {} bytes",
                                (self.address_cells + self.size_cells) * TOKEN_SIZE as u32,
                                property_len
                            );
                            /* Some devices contains the dtb which does not match the specifications... */
                        }
                    }
                    *pointer += property_len as usize;
                    Self::skip_padding(pointer)?;
                }
                FDT_BEGIN_NODE => {
                    if let Some(index) = compatible_index {
                        return Ok(Some((self.clone(), index)));
                    }
                    let mut child = self.clone();
                    child.base_pointer = *pointer;
                    let result =
                        child._search_device_by_compatible(compatible_devices, dtb, pointer)?;
                    if result.is_some() {
                        return Ok(result);
                    }
                }
                FDT_NOP => {}
                _ => {
                    println!(
                        "Expected TOKEN, but found {:#X}(Offset from current node: {:#X})",
                        u32::from_be(unsafe { *(*pointer as *const u32) }),
                        *pointer - self.base_pointer
                    );
                    return Err(());
                }
            }
            Self::skip_nop(pointer);
        }
        if let Some(index) = compatible_index {
            return Ok(Some((self.clone(), index)));
        }
        *pointer += TOKEN_SIZE;
        return Ok(None);
    }

    pub fn get_search_holder(&self) -> Result<DtbNodeNameSearchHolder, ()> {
        let mut pointer = self.base_pointer;
        Self::skip_nop(&mut pointer);
        if unsafe { *(pointer as *const u32) } != FDT_BEGIN_NODE {
            println!(
                "Expected FDT_BEGIN_NODE, but found {:#X}",
                u32::from_be(unsafe { *(pointer as *const u32) })
            );
            return Err(());
        }
        pointer += TOKEN_SIZE;

        while unsafe { *(pointer as *const u8) } != 0 {
            pointer += 1;
        }
        pointer += 1;
        Self::skip_padding(&mut pointer)?;

        Ok(DtbNodeNameSearchHolder {
            node: self.clone(),
            pointer,
        })
    }

    pub fn is_status_okay(&self, dtb: &DtbAnalyser) -> Result<Option<bool>, ()> {
        let mut s = self.clone();
        if let Some(p) = s.search_pointer_to_property(PROP_STATUS, dtb)? {
            Ok(Some(Self::match_string(p, PROP_STATUS_OKAY)))
        } else {
            Ok(None)
        }
    }

    #[allow(dead_code)]
    pub fn get_reg(&self, dtb: &DtbAnalyser) -> Result<Option<u64>, ()> {
        let mut s = self.clone();
        if let Some(mut _p) = s.search_pointer_to_property(PROP_REG, dtb)? {
            unimplemented!()
        } else {
            Ok(None)
        }
    }

    pub fn get_offset(&self) -> usize {
        self.address_offset
    }
}

impl DtbNodeNameSearchHolder {
    #[allow(dead_code)]
    pub fn search_next_device_by_node_name(
        &mut self,
        node_name: &[u8],
        dtb: &DtbAnalyser,
    ) -> Result<Option<DtbNode>, ()> {
        let result =
            self.node
                .__search_device_by_node_name(node_name, dtb, &mut self.pointer, false)?;
        if let Some(t) = &result {
            self.pointer = t.base_pointer;
            DtbNode::skip_to_end_of_node(&mut self.pointer)?;
        } else {
            if unsafe { *(self.pointer as *const u32) } != FDT_END {
                if self.pointer >= dtb.get_struct_block_limit() {
                    println!("Broken DTB");
                    return Err(());
                }
                self.node = dtb.get_root_node();
                self.node.base_pointer = self.pointer;
                return self.search_next_device_by_node_name(node_name, dtb);
            }
        }
        return Ok(result);
    }

    pub fn search_next_device_by_compatible(
        &mut self,
        compatible_devices: &[&[u8]],
        dtb: &DtbAnalyser,
    ) -> Result<Option<(DtbNode, usize)>, ()> {
        let result =
            self.node
                .__search_device_by_compatible(compatible_devices, dtb, &mut self.pointer)?;
        if let Some((t, _)) = &result {
            self.pointer = t.base_pointer;
            DtbNode::skip_to_end_of_node(&mut self.pointer)?;
        } else {
            if unsafe { *(self.pointer as *const u32) } != FDT_END {
                if self.pointer >= dtb.get_struct_block_limit() {
                    println!("Broken DTB");
                    return Err(());
                }
                self.node = dtb.get_root_node();
                self.node.base_pointer = self.pointer;
                return self.search_next_device_by_compatible(compatible_devices, dtb);
            }
        }
        return Ok(result);
    }
}

impl DtbAnalyser {
    pub fn new(base_address: usize) -> Result<Self, ()> {
        let dtb_header = unsafe { &*(base_address as *const DtbHeader) };
        if !dtb_header.check_magic() {
            println!("Failed to check magic code.");
            return Err(());
        }
        Ok(Self {
            struct_block_address: base_address + u32::from_be(dtb_header.off_dt_struct) as usize,
            struct_block_size: u32::from_be(dtb_header.size_dt_struct) as usize,
            strings_block_address: base_address + u32::from_be(dtb_header.off_dt_strings) as usize,
            strings_block_size: u32::from_be(dtb_header.size_dt_strings) as usize,
        })
    }

    pub fn get_root_node(&self) -> DtbNode {
        DtbNode {
            address_offset: 0,
            address_cells: DEFAULT_ADDRESS_CELLS,
            size_cells: DEFAULT_SIZE_CELLS,
            base_pointer: self.struct_block_address,
        }
    }

    fn get_name(&self, offset_of_segments: u32) -> Result<usize, ()> {
        if self.strings_block_size > offset_of_segments as usize {
            Ok(self.strings_block_address + offset_of_segments as usize)
        } else {
            Err(())
        }
    }

    fn get_struct_block_limit(&self) -> usize {
        self.struct_block_size + self.struct_block_address
    }
}
