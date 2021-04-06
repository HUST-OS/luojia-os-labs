target := "riscv64imac-unknown-none-elf"
mode := "debug"
build-path := "target/" + target + "/" + mode + "/"

kernel-elf := build-path + "batch-kernel"
kernel-bin := build-path + "batch-kernel.bin"

build:
    @just -f "01-batch-kernel/justfile" build
