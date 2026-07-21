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

  .tdata : {
      _tdata_start = .;       /* Define a symbol at the start of initialized TLS */
      *(.tdata .tdata.*)
      _tdata_end = .;         /* Define a symbol at the end of initialized TLS */
  }

  .tbss : {
      _tbss_start = .;         /* Define a symbol at the start of zeroed TLS */
      *(.tbss .tbss.*)
      *(.tcommon)
      _tbss_end = .;           /* Define a symbol at the end of zeroed TLS */
  }
  . = ALIGN(16);
  _boot_stack_bottom = .;
  . += 0x20000; /* 128KB stack, FDT needs 5.5KB for map */
  _boot_stack_top = .;
  _end = .;
}