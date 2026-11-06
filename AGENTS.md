# Notes for AI agents: building and running Doors OS with `builder`

Doors OS is not built directly with `cargo`. It is built through a two-stage pipeline:

1. **`builder/`** — a small Rust program that *generates* a `CMakeLists.txt` (plus a few
   supporting config files) from a declarative TOML machine config.
2. **CMake/Make** — the generated build system actually invokes `cargo`, `grub-mkrescue`,
   `grub-mknetdir`, the emulators, etc.

`builder` never compiles the kernel itself. If you are looking for "the build command",
the real answer is "regenerate the CMake files, then run the right `make` target".

Source of truth: `builder/src/main.rs` (config schema + generator),
`builder/src/{qemu,bochs,virtualbox}.rs` (per-emulator rules).

---

## Quick start (from the repository root)

```sh
# 1. Generate the build system for a machine/emulator combo
cargo run -p builder -- --name configs/pc64_qemu.toml

# 2. Configure (in-source!) and build + boot
cmake .
make run
```

Everything must be run **from the repository root**. `builder` hardcodes relative paths
(`./kernel`, `./user`, `./build/iso`, `./local_config.toml`, `./CMakeLists.txt`), and the
CMake build is **in-source** — `Makefile`, `CMakeFiles/`, `CMakeCache.txt` land in the repo
root (all gitignored).

`make help` prints the full list of generated targets.

---

## The two config files (easy to confuse)

| File | Role | Committed? |
| --- | --- | --- |
| `configs/<machine>.toml` | **OS/machine config** — passed via `--name`. Selects kernel machine, disk image type, emulator. | yes |
| `local_config.toml` | **Host config** — machine-specific binary paths, serial ports, network devices. Path is hardcoded; there is no flag for it. | no (create your own) |

`--name` is a *path*, resolved relative to the current directory, so pass
`configs/pc64_qemu.toml`, not `pc64_qemu`. If the file is missing, `builder` prints the
valid entries in that directory; if `local_config.toml` is missing it writes a file full of
placeholder paths (`./optional/example/qemu/path/here`, `./example/grub/source`, ...) and
uses it. **Those placeholders are not valid** — fill them in after the first run.

`configs/` currently provides: `pc32_{bochs,qemu,virtualbox,pxe}`,
`pc64_{bochs,qemu,virtualbox,pxe}`, `pc64_qemu_nonet`, `stm32f769i-disco`.

### OS config shape (`DoorsConfiguration`)

```toml
kernel_machine = "x86_64-unknown-none"   # cargo target used to build the kernel
user_machine   = "x86_64-unknown-doors"  # cargo target used to build user programs
kernel_path    = "x86_64-unknown-none"   # target/ subdir the disk image copies from
disassembly    = true

[kernel_config]                          # forwarded verbatim to kernel/config.toml
machine_name = "pc64"
acpi = true
gdbstub = false        # must be true for `make gdb` (kernel-side gdbstub)
mm_debug = false
test = false           # true => the self-test kernel variety
network = true
modules = ["hpet", "intelpro1000"]

[disk.unique.Cd]                         # or [disk.unique.Network]
kernel_path = "./build/iso/boot/kernel"

[disk.common]
output = "cd64.iso"
disk_label = "DoorsOS"
config_files = [["grub2.lst", "./build/iso/boot/grub/grub.cfg"]]
# grub_config = "grub2pxe.lst"           # optional override of the generated grub.cfg

[target.config]
net_devs = [0]         # indices into local_config.toml [[net_devs]]
serial_ports = [0, 2]  # indices into local_config.toml serial_ports

[target.emulator.Qemu]                   # or .Bochs / .VirtualBox / .None (no run rules)
```

Note the indirection: `[target.config]` selects *indexes* into the host's device lists, so
the same machine config works on different hosts. Emulator-specific structs
(`Qemu`/`Bochs`/`VirtualBox`) currently carry no fields.

---

## What running `builder` writes

| Path | Notes |
| --- | --- |
| `CMakeLists.txt` | **Generated. Never hand-edit** — your edits vanish on the next run. Edit `builder/src/` instead. |
| `kernel/config.toml` | Generated from `[kernel_config]`; the kernel's own `build.rs` reads it. Gitignored. |
| `kernel/.vscode/settings.json` | Sets `rust-analyzer.cargo.target` to `kernel_machine`. |
| `gdb_config.gdb`, `gdb_stub.gdb` | QEMU / VirtualBox gdb scripts. |
| `bochs_config.txt` | Bochs config. |
| `./ready_kernel` etc. | **Deleted** on purpose (see the clean gotcha below). |

Because `CMakeLists.txt`, `Makefile`, `CMakeFiles/`, `CMakeCache.txt`, `kernel/config.toml`
and `disassemble.txt` are all gitignored, a freshly cloned repo has **no build system at
all** until you run `builder` + `cmake .`. Conversely, a `CMakeLists.txt` sitting in the
tree is *stale* and was generated for some other config — do not trust it as evidence of
how the current config builds (e.g. the one in the working tree is a `thumbv7em` / PXE /
no-emulator config, while the leftover root `Makefile` still advertises `run`/`debug`/`gdb`
targets from an earlier QEMU run).

---

## Generated make targets

Always present: `kernel`, `user`, `boot_disk`, `disassemble`, `fmt`, `stack`,
`kernel_clippy`, `kernel_fmt`, `kernel_expand`, `kernel_volatility3_symbols`,
`user_clippy`, `user_fmt`, `rust_compiler`.

Emulator-dependent:

| Target | QEMU | Bochs | VirtualBox | None |
| --- | --- | --- | --- | --- |
| `run` | yes | yes (also needs `debug_symbols_kernel`) | yes | – |
| `debug` | yes (`-s -S`, waits at gdb :1234) | – | yes (`--dbg --debug`) | – |
| `debug2` | `gdb -x gdb_config.gdb` | – | – | – |
| `gdb` | `gdb -x gdb_stub.gdb` | – | `gdb -x gdb_config.gdb` | – |

`run`/`debug`/`gdb` `DEPENDS` on `boot_disk` (and usually `disassemble`), so a single
`make run` builds the kernel, userland and the ISO for you. There is no `buildscript`
target any more despite what `README.md` says — re-run `cargo run -p builder -- --name ...`
to regenerate.

Kernel (`cargo +nightly build --release --target <kernel_machine>`) and userland
(`cargo build --release --target <user_machine>`) are built as custom targets in
`./kernel` and `./user`; both crates are excluded from the root workspace and pin their own
toolchains (`kernel/rust-toolchain.toml` → nightly, `user/rust-toolchain.toml` →
`doors-user`, a locally built toolchain created by `make rust_compiler` from `./rust`).

---

## Gotchas worth knowing before you debug a build

- **`builder` forces a clean rebuild.** It deletes `ready_kernel`/`ready_user`; those are
  custom-command outputs whose recipe runs `cargo clean` in `./kernel` and `./user`. So the
  first `make` after any `builder` run is a full rebuild, by design.
- **`builder` with a VirtualBox target has side effects outside the repo.** Its
  `build_config` immediately shells out to `VBoxManage`: it `unregistervm doors-os-64
  --delete`s and recreates the `doors-os-64` VM and reconfigures uarts/NICs/storage on
  every run. Do not run it casually on someone's machine.
- **Serial/network indices are host-specific.** If you change `local_config.toml` list
  order, every `configs/*.toml` pointing at those indexes silently changes meaning.
- **Two different debug paths for QEMU.** `make debug` + `make debug2` attach gdb to QEMU's
  gdbserver (`:1234`); `make gdb` attaches to the *kernel's own* gdbstub, which is exposed
  as a `TcpServer` serial port (`:12345`) and requires `gdbstub = true` in `[kernel_config]`.
- **Disk-image prerequisites.** CD images need `grub-mkrescue` (+ `xorriso`, `mtools`) and
  `rust-strip` on `PATH`; PXE/network images need `grub-mknetdir` and a valid `grub_source`
  in `local_config.toml` (e.g. `/usr/lib/grub/i386-pc`). Network images also need
  `tftp_base` for `make boot_disk_deploy`.
- **QEMU is launched with `-m 8`** (8 MB) and a hardcoded `-device e1000`; `tap` netdevs
  need an existing tap device and `/etc/qemu-ifup`, so use a `*_nonet` config when you just
  want a quick boot.
- **Config parse failures are noisy.** A schema mismatch prints `Need to check in path ...`
  and then panics with `Invalid kernel configuration`; a missing file prints the directory
  listing. Read the printed listing rather than guessing config names.
- **Windows paths are only half-supported.** The network (`PXE`) disk builder hits
  `todo!()` on Windows; CD images on Windows are produced by `cargo run --bin imager`
  instead of `grub-mkrescue`.

---

## Minimal loop for iterating on kernel code

```sh
cargo run -p builder -- --name configs/pc64_qemu.toml   # only when config/build rules change
cmake .                                                 # re-configure after builder
cd kernel && cargo +nightly build --release --target x86_64-unknown-none --bin kernel
                                                        # fast check without a full clean
make boot_disk && make run                              # or: make debug / make gdb
make disassemble    # -> disassemble.txt
make stack          # -> stack.txt
```

Serial output from the emulator is written to `serial.log` / `serial2.log` (or exposed on
TCP `12345`) according to `local_config.toml` — that is where kernel `print!` output goes,
and it is the first place to look when a boot fails silently.
