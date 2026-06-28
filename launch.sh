if (cargo build --release) then
  cargo objcopy --release -- -O binary os.bin

  # if ($1 = "-d") then
  #   qemu-system-riscv32 \
  #     -machine virt,dumpdtb=virt.dtb \
  #     -bios none \
  #     -kernel os.bin \
  #     -device virtio-gpu-pci,xres=640,yres=480,hostmem=1G \
  #     -display cocoa \
  #     -serial stdio \
  # else
    qemu-system-riscv32 \
      -machine virt \
      -bios none \
      -kernel os.bin \
      -device virtio-gpu-device,xres=640,yres=480 \
      -display cocoa \
      -serial stdio
else
  echo "Build failed"
  exit 1
fi
