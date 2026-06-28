use crate::mm::{
    // map::{UART0_END, UART0_SIZE, UART0_START},
    mem_read,
    mem_write,
};
const UART0_START: u32 = 0x1000_0000;

pub fn write_char(char: char) {
    unsafe {
        mem_write(UART0_START, char as u8);
    }
}

pub fn write_byte(b: u8) {
    unsafe {
        mem_write(UART0_START, b);
    }
}
