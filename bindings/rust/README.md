# unicornafl-rs

Rust bindings for the [Unicorn](http://www.unicorn-engine.org/) emulator with AFL++ extensions and utility functions.

```rust
use unicornafl::arm::Register;
use unicornafl::consts::{Arch, Mode, Permission, SECOND_SCALE};

fn main() {
    let arm_code32: Vec<u8> = vec![0x17, 0x00, 0x40, 0xe2]; // sub r0, #23

    let mut unicorn = unicornafl::Unicorn::new(Arch::ARM, Mode::LITTLE_ENDIAN, 0).expect("failed to initialize Unicorn instance");
    let mut emu = unicorn.borrow();
    emu.mem_map(0x1000, 0x4000, Permission::ALL).expect("failed to map code page");
    emu.mem_write(0x1000, &arm_code32).expect("failed to write instructions");

    emu.reg_write(Register::R0 as i32, 123).expect("failed write R0");
    emu.reg_write(Register::R5 as i32, 1337).expect("failed write R5");

    let _ = emu.emu_start(0x1000, (0x1000 + arm_code32.len()) as u64, 10 * SECOND_SCALE, 1000);
    assert_eq!(emu.reg_read(Register::R0 as i32), Ok(100));
    assert_eq!(emu.reg_read(Register::R5 as i32), Ok(1337));
}
```

The bindings offer access to AFL++'s API:
```rust
emu.emu_start(0x1fe741, 0x001ff106, 0, 1).expect("failed to kick off emulation");

let ret = emu.afl_fuzz(
        input_file,
        Box::new(place_input_callback),
        &[0x001ff106, 0x001ff0aa],  // exit addresses
        Box::new(crash_validation_callback),
        true,                       // always validate
        100                         // rounds in persistent mode
    );
```

## Installation

This project has been tested on Linux, OS X and Windows.
The bindings are built for version 1.0 of unicorn.

First, build AFL++ and Unicorn mode.

To use unicornafl-rs, simply add it as a dependency to the Cargo.toml of your program.

```
[dependencies]
unicornafl = { path = "/path/to/bindings/rust", version="1.0.0" }
```

## Acknowledgements

These bindings are based on Sébastien Duquette's (@ekse) [unicorn-rs](https://github.com/unicorn-rs/unicorn-rs).
We picked up the project, as it is no longer maintained.
Thanks to all contributers.


## Contributing

Contributions to this project are appreciated. Pull requests, bug reports, code review, tests,
documentation or feedback on your use of the bindings, everything is appreciated.
If you have any questions, please feel free to open an issue.
