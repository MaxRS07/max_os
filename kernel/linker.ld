/* linker.ld */
OUTPUT_ARCH( "riscv" )
ENTRY( _start )

SECTIONS
{
  /* QEMU Virt Board start ram */
  . = 0x80000000;

  .text : {
    *(.text._start)
    *(.text .text.*)
  }
  /DISCARD/ : { *(.eh_frame) *(.eh_frame_hdr) }
  .rodata : { *(.rodata .rodata.*) }
  .data   : { *(.data .data.*) }
  .bss    : { *(.bss .bss.*) }
  . = ALIGN(16);
  . += 0x4000; /* 16KB stack */
  _stack_top = .;
  _end = .;
}