[[ "$1" == "-d" ]] && DEBUG=1 || DEBUG=0
[[ "$2" == "-mt" ]] && REMOUNT=1 || REMOUNT=0 # Forces the disk to remount all drives, overwriting current metadata. This is useful if you have changed the partition table or filesystem on the disk and want to ensure that the changes are reflected in the virtual machine.

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
    -drive file=/Volumes/MAX_OS/max_os.img,format=raw,id=usb_backend,if=none \
    -device virtio-blk-device,drive=usb_backend \
    -display cocoa \
    -serial stdio \
    -append "$debug={DEBUG}" \
    -append "$remount={REMOUNT}"
else
  echo "Build failed"
  exit 1
fi

