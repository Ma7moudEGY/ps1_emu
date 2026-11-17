# PS1 Emulator — Build Plan

**Language:** Rust (edition 2024, single binary crate)
**Windowing/rendering:** `winit` + `glow` (OpenGL), dependencies added only as needed
**Primary doc:** `docs/guide.PDF` — *Playstation Emulation Guide*, Lionel Flandrin (Rust-based, step-by-step)
**Reference doc:** `docs/psx-spx.pdf` — *PSX Specifications*, Martin Korth (full hardware reference)

---

## 1. Target file structure

```
ps1_emu/
├── Cargo.toml                 # edition 2024 (already set); deps added as needed
├── docs/                      # existing PDFs
└── src/
    ├── main.rs                # entry point: load BIOS → build Interconnect → run loop
    │
    ├── memory/
    │   ├── mod.rs             # memory map, address decoding, regions (KUSEG etc.)
    │   ├── ram.rs             # 2MB main RAM (+1KB scratchpad later)
    │   └── bios.rs            # BIOS ROM (loads SCPH1001.bin from disk)
    │
    ├── cpu/
    │   ├── mod.rs             # PC, fetch/decode/execute loop, delay slot state
    │   ├── decode.rs          # extract op/rs/rt/rd/funct/imm fields from 32-bit word
    │   ├── instructions.rs    # one impl at a time: LUI, ORI, SW, J, BNE...
    │   ├── exceptions.rs      # exception entry, COP0 cause/epc, RFE
    │   └── icache.rs          # instruction cache (guide's last chapter)
    │
    ├── cop0.rs                # SR, CAUSE, EPC, PRID, JPC/DIC (MFC0/MTC0)
    │
    ├── interconnect.rs        # the Interconnect (guide's name — match it): owns
    │                          # RAM/BIOS/GPU/DMA/timers, decodes addresses,
    │                          # routes loads/stores & interrupts
    ├── interrupts.rs          # I_STAT / I_MASK
    ├── dma.rs                 # DPCR/DICR, 7 channels, OTC, block copy, linked lists
    ├── timers.rs              # 3 root counters
    │
    ├── gpu/
    │   ├── mod.rs             # GPUSTAT, VRAM (1MB u16 array), GPUREAD
    │   ├── commands.rs        # GP0/GP1 command state machine
    │   └── renderer.rs        # glow/GL renderer — quadrilaterals first, boot logo
    │
    ├── debugger.rs            # breakpoints, watchpoints, stepping, disassembly
    │
    └── frontend/
        ├── mod.rs             # winit event loop, keyboard input, clean exit
        └── video.rs           # GL context, surface/swapchain, texture upload
```

**Not in the tree yet** (add when outgrowing the guide, referenced from `psx-spx`):
`cdrom.rs`, `spu.rs`, `mdec.rs`, `controller.rs` (pad via serial port), GPIO timer bits.

---

## 2. Implementation order (~8 milestones, mirrors the guide)

**Status: milestone 1 complete (skeleton compiles, module tree in place).**

> Structure note: the emulator's central bus object is named `interconnect.rs`
> to match the guide's terminology ("Interconnect") — search the guide using
> that name.

| # | Milestone | Guide chapters | Visible result |
|---|-----------|---------------|----------------|
| 1 | Skeleton: module tree, stub structs, `cargo build` green | — | compiles |
| 2 | BIOS loads, memory map, PC reset, first `LUI`/`ORI` | CPU pt. 1 (pp. 9–20) | runs first real instructions |
| 3 | Instruction-by-instruction growth (run BIOS, implement what it hits next) | CPU pt. 2–3 | CPU survives deep into BIOS |
| 4 | COP0 + exceptions + timers + interrupts | CPU pt. 3 (exceptions pp. 63–73) | BIOS handles exceptions |
| 5 | DMA: OTC + block copy + linked list channels — **includes the guide's refactor pass: rewrite loads/stores with generic traits, port CPU/interconnect/RAM/BIOS code** (pp. 158–164) | DMA + Interconnect generic loads/stores | GPU receives commands |
| 6 | GPU state machine + GP0/GP1 commands | GPU pt. 1–2 | VRAM fills correctly |
| 7 | winit + glow renderer — **guide uses SDL2 + OpenGL: read for concepts, translate to winit/glow API** (window/event loop/GL context/shaders/VAO map 1:1) | GPU pt. 3 (pp. 136–157) | **Sony logo renders 🎉** |
| 8 | Debugger + instruction cache — **last guide milestone** | final chapters (pp. 164–171) | step/troubleshoot for the next phase |
| 9+ | Beyond the guide — **psx-spx only, no more chapter support:** CDROM, SPU, controllers, MDEC, booting actual games | — | games run |

> **Note:** the guide implements instructions one at a time, driven by BIOS
> crashes into unimplemented code — follow that literally in milestone 3, don't
> front-load an instruction table. Milestones 5–8 rewrite earlier code by
> design (the guide's "porting" passes): milestone 3's code is not meant to
> survive in its original form.

---

## 3. Rust conventions (for a first-timer)

- One module = one file; declare with `mod` in `main.rs`; `snake_case` filenames.
- `u32`/`u16`/`u8` everywhere, explicit; use wrapping ops for CPU arithmetic
  (Rust panics on overflow in debug builds — maps directly to the guide's ADDI
  overflow chapter).
- The guide deliberately refactors its code twice (generic loads/stores, porting
  pass) — do the same; it's a great way to learn Rust's traits and generics.
- Unit tests live in the same file under `#[cfg(test)]`, run with `cargo test`.
- Read slowly: branch delay slots (p. 29), load delay slots (p. 37), and the
  exceptions chapter (pp. 63–73) hold most first-emulator bugs.
- "Type safety in the register interface" (p. 31) is an early Rust lesson in
  newtype wrappers — good practice for `impl` and traits.

---

## 4. Things needed outside of code

- **PS1 BIOS ROM file** (e.g. `SCPH1001.bin`) — copyrighted; dump it from your
  own console. Emulator takes its path as an argument. Never commit it.
- Add a `.gitignore` entry for `*.bin` in milestone 1.
