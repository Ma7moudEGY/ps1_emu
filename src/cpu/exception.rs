pub enum Exception {
    LoadAddressError = 0x4,
    StoreAddressError = 0x5,
    SysCall = 0x8,
    Break = 0x9,
    IllegalInstruction = 0xa,
    CoprocessorError = 0xb,
    Overflow = 0xc,
}