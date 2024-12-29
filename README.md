# Build and flash
install Rust (via [rustup](https://www.rust-lang.org/tools/install)) and the esp-rs toolchain (via [espup](https://docs.esp-rs.org/book/installation/riscv-and-xtensa.html)).

connect your board, build and flash with:
```bash
cargo run --release
```
# TO-DO
- support motors using ESP32 MCPWM ✅
- support dual motors ✅
- support changing direction for motors ✅
- add ultrasonic sensor support ✅
- add dual ultrasonic sensor support ✅
- simplify codebase, remove bullshit
    - simplify motor module ✅
    - simplify ultrasonic module ✅

# Project Overview
`todo!();`

# Resources
- [Async Rust in Embedded Systems with Embassy - Dario Nieuwenhuis](https://www.youtube.com/watch?v=H7NtzyP9q8E): excellent introduction to embassy and async flow in embedded systems
- [esp-hal examples](https://github.com/esp-rs/esp-hal/tree/main/examples/): took many ideas from here, the docs are really good also.
- [Embassy Book](https://embassy.dev/book/): lots of info