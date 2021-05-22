OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
    . = 0x1000;
    .text : ALIGN(4K) {
        *(.text.entry)
        *(.text .text.*)
    }
    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }
    .data : ALIGN(4K) {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }
    .bss : ALIGN(4K) {
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        ebss = .;
    }
    /DISCARD/ : {
        *(.eh_frame)
    }
}
