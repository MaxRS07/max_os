ARGS=()

# Loop through all arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    -d|--debug)
      ARGS+=("debug=1")
      shift
      ;;
    # Forces the disk to remount all drives, overwriting current metadata.
    -mt|--remount)
      ARGS+=("remount=1")
      shift
      ;;
    *)
      # Handle unknown arguments or positional parameters if needed
      shift
      ;;
  esac
done

JOIN_ARGS=$(IFS=,; echo "${ARGS[*]}")

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
    -device virtio-net-device,netdev=net0 \
    -netdev user,id=net0,hostfwd=tcp::10022-:22 \
    -display cocoa \
    -serial stdio \
    -append "${JOIN_ARGS}"
else
  echo "Build failed"
  exit 1
fi

