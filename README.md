# Build and flash
install Rust (via [rustup](https://www.rust-lang.org/tools/install)) and the esp-rs toolchain (via [espup](https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html)).

connect your board, build and flash with:
```bash
cargo run --release
```
# TO-DO
- support dual motors using ESP32 MCPWM ✅
- add ultrasonic sensor support 
- simplify codebase, remove bullshit