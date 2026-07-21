use crate::fdt_header::FdtHeader;

#[derive(Clone, Copy, Debug)]
pub struct FdtStream {
    ptr: *const u32,
    strings_ptr: *const u8,
}

impl FdtStream {
    pub fn new(header: &FdtHeader) -> Self {
        Self {
            ptr: (header.addr + header.off_dt_struct as usize) as *const u32,
            strings_ptr: (header.addr + header.off_dt_strings as usize) as *const u8,
        }
    }
    /// Advances until a property with name `name` is found or stream ends.
    /// ## Returns
    /// address of the data inside the named tag:
    /// `Some(*const u8)` if a matching name is found, else `None`
    /// ## Safety
    ///
    pub unsafe fn seek_named_property(&mut self, name: &'static str) -> Option<FdtElement> {
        loop {
            let element = unsafe { self.next_element() };
            match element {
                Some(FdtElement::Property {
                    name: prop_name,
                    value_ptr,
                    len,
                }) => {
                    if name == prop_name {
                        return Some(FdtElement::Property {
                            name,
                            value_ptr,
                            len,
                        });
                    }
                }
                Some(_) => {}
                None => break,
            }
        }
        None
    }

    /// Goes to the next structural element in the device tree
    /// # Safety
    ///
    pub unsafe fn next_element(&mut self) -> Option<FdtElement> {
        loop {
            let token = unsafe { self.ptr.read_volatile().swap_bytes() };
            match token {
                // FDT_BEGIN_NODE
                0x0000_0001 => {
                    unsafe {
                        self.ptr = self.ptr.add(1);
                        let mut byte_ptr = self.ptr as *const u8;
                        let mut len = 0;
                        while *byte_ptr != 0 {
                            len += 1;
                            byte_ptr = byte_ptr.add(1);
                        }
                        len += 1;

                        let name_slice =
                            core::slice::from_raw_parts(self.ptr as *const u8, len - 1);
                        let mut name = core::str::from_utf8(name_slice).unwrap_or("");
                        if name.is_empty() {
                            name = "root" // for display purposes
                        }

                        // align to 4 byte (u32)
                        let aligned_bytes = (len + 3) & !3;
                        self.ptr = (self.ptr as *const u8).add(aligned_bytes) as *const u32;

                        return Some(FdtElement::BeginNode { name });
                    }
                }
                // FDT_PROP
                0x0000_0003 => unsafe {
                    let data_len = self.ptr.add(1).read_volatile().swap_bytes();
                    let name_offset = self.ptr.add(2).read_volatile().swap_bytes();

                    let name_ptr = self.strings_ptr.add(name_offset as usize);
                    let mut len = 0;
                    while *name_ptr.add(len) != 0 {
                        len += 1;
                    }
                    let name_slice = core::slice::from_raw_parts(name_ptr, len);
                    let name = core::str::from_utf8(name_slice).unwrap_or("");

                    let value_ptr = self.ptr.add(3) as *const u8;

                    let aligned_data_len = (data_len as usize + 3) & !3;
                    // FIX: Advance past token (1), data_len (1), name_offset (1), and value data length
                    self.ptr = self.ptr.add(3).add(aligned_data_len / 4);

                    return Some(FdtElement::Property {
                        name,
                        value_ptr,
                        len: data_len as usize,
                    });
                },
                // FDT_END_NODE or FDT_NOP
                0x0000_0002 | 0x0000_0004 => unsafe {
                    // FIX: You must advance the pointer past this 4-byte token before returning!
                    self.ptr = self.ptr.add(1);

                    // Map the token types properly if your FdtElement supports distinct variants
                    if token == 0x0000_0002 {
                        return Some(FdtElement::EndNode);
                    }
                    // If it's a NOP (0x4), don't return! Let the loop continue to read the next token.
                },
                // FDT_END
                0x0000_0009 => {
                    unsafe {
                        self.ptr = self.ptr.add(1);
                    }
                    return None;
                }
                _ => self.ptr = unsafe { self.ptr.add(1) },
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FdtElement {
    BeginNode {
        name: &'static str,
    },
    Property {
        name: &'static str,
        value_ptr: *const u8,
        len: usize,
    },
    EndNode,
}
