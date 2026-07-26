[[ "$1" == "-d" ]] && DEBUG=1 || DEBUG=0

if (RUSTFLAGS="-Awarnings" cargo build --release) then
  cargo objcopy --release -- -O binary os.bin
  qemu-system-riscv32 \
    -m 1G \
    -machine virt \
    -bios none \
    -kernel os.bin \
    -global virtio-mmio.force-legacy=false \
    -device virtio-gpu-device,xres=640,yres=480 \
    -device virtio-keyboard-device \
    -device virtio-tablet-device \
    -drive file=/Volumes/MaxOS/max_os.img,format=raw,id=usb_backend,if=none \
    -device virtio-blk-device,drive=usb_backend \
    -display cocoa \
    -serial stdio \
    -append "${DEBUG}"
else
  echo "Build failed"
  exit 1
fi
