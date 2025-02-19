#![allow(non_camel_case_types)]
// For Unicorn Engine. AUTO-GENERATED FILE, DO NOT EDIT

// MIPS registers
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Register {
    INVALID = 0,

    // General purpose registers
    PC = 1,
    GPR0 = 2,
    GPR1 = 3,
    GPR2 = 4,
    GPR3 = 5,
    GPR4 = 6,
    GPR5 = 7,
    GPR6 = 8,
    GPR7 = 9,
    GPR8 = 10,
    GPR9 = 11,
    GPR10 = 12,
    GPR11 = 13,
    GPR12 = 14,
    GPR13 = 15,
    GPR14 = 16,
    GPR15 = 17,
    GPR16 = 18,
    GPR17 = 19,
    GPR18 = 20,
    GPR19 = 21,
    GPR20 = 22,
    GPR21 = 23,
    GPR22 = 24,
    GPR23 = 25,
    GPR24 = 26,
    GPR25 = 27,
    GPR26 = 28,
    GPR27 = 29,
    GPR28 = 30,
    GPR29 = 31,
    GPR30 = 32,
    GPR31 = 33,

    // DSP registers
    DSPCCOND = 34,
    DSPCARRY = 35,
    DSPEFI = 36,
    DSPOUTFLAG = 37,
    DSPOUTFLAG16_19 = 38,
    DSPOUTFLAG20 = 39,
    DSPOUTFLAG21 = 40,
    DSPOUTFLAG22 = 41,
    DSPOUTFLAG23 = 42,
    DSPPOS = 43,
    DSPSCOUNT = 44,

    // ACC registers
    AC0 = 45,
    AC1 = 46,
    AC2 = 47,
    AC3 = 48,

    // COP registers
    CC0 = 49,
    CC1 = 50,
    CC2 = 51,
    CC3 = 52,
    CC4 = 53,
    CC5 = 54,
    CC6 = 55,
    CC7 = 56,

    // FPU registers
    F0 = 57,
    F1 = 58,
    F2 = 59,
    F3 = 60,
    F4 = 61,
    F5 = 62,
    F6 = 63,
    F7 = 64,
    F8 = 65,
    F9 = 66,
    F10 = 67,
    F11 = 68,
    F12 = 69,
    F13 = 70,
    F14 = 71,
    F15 = 72,
    F16 = 73,
    F17 = 74,
    F18 = 75,
    F19 = 76,
    F20 = 77,
    F21 = 78,
    F22 = 79,
    F23 = 80,
    F24 = 81,
    F25 = 82,
    F26 = 83,
    F27 = 84,
    F28 = 85,
    F29 = 86,
    F30 = 87,
    F31 = 88,
    FCC0 = 89,
    FCC1 = 90,
    FCC2 = 91,
    FCC3 = 92,
    FCC4 = 93,
    FCC5 = 94,
    FCC6 = 95,
    FCC7 = 96,

    // AFPR128
    W0 = 97,
    W1 = 98,
    W2 = 99,
    W3 = 100,
    W4 = 101,
    W5 = 102,
    W6 = 103,
    W7 = 104,
    W8 = 105,
    W9 = 106,
    W10 = 107,
    W11 = 108,
    W12 = 109,
    W13 = 110,
    W14 = 111,
    W15 = 112,
    W16 = 113,
    W17 = 114,
    W18 = 115,
    W19 = 116,
    W20 = 117,
    W21 = 118,
    W22 = 119,
    W23 = 120,
    W24 = 121,
    W25 = 122,
    W26 = 123,
    W27 = 124,
    W28 = 125,
    W29 = 126,
    W30 = 127,
    W31 = 128,
    HI = 129,
    LO = 130,
    P0 = 131,
    P1 = 132,
    P2 = 133,
    MPL0 = 134,
    MPL1 = 135,
    MPL2 = 136,
    CP0_CONFIG3 = 137,
    CP0_USERLOCAL = 138,
    CP0_STATUS = 139,
    ENDING = 140,
}

impl Register {
    // alias registers
    // (assoc) ZERO = 2,
    // (assoc) AT = 3,
    // (assoc) V0 = 4,
    // (assoc) V1 = 5,
    // (assoc) A0 = 6,
    // (assoc) A1 = 7,
    // (assoc) A2 = 8,
    // (assoc) A3 = 9,
    // (assoc) T0 = 10,
    // (assoc) T1 = 11,
    // (assoc) T2 = 12,
    // (assoc) T3 = 13,
    // (assoc) T4 = 14,
    // (assoc) T5 = 15,
    // (assoc) T6 = 16,
    // (assoc) T7 = 17,
    // (assoc) S0 = 18,
    // (assoc) S1 = 19,
    // (assoc) S2 = 20,
    // (assoc) S3 = 21,
    // (assoc) S4 = 22,
    // (assoc) S5 = 23,
    // (assoc) S6 = 24,
    // (assoc) S7 = 25,
    // (assoc) T8 = 26,
    // (assoc) T9 = 27,
    // (assoc) K0 = 28,
    // (assoc) K1 = 29,
    // (assoc) GP = 30,
    // (assoc) SP = 31,
    // (assoc) FP = 32,
    // (assoc) S8 = 32,
    // (assoc) RA = 33,
    // (assoc) HI0 = 45,
    // (assoc) HI1 = 46,
    // (assoc) HI2 = 47,
    // (assoc) HI3 = 48,
    // (assoc) LO0 = 45,
    // (assoc) LO1 = 46,
    // (assoc) LO2 = 47,
    // (assoc) LO3 = 48,
    pub const ZERO: Register = Register::GPR0;
    pub const AT: Register = Register::GPR1;
    pub const V0: Register = Register::GPR2;
    pub const V1: Register = Register::GPR3;
    pub const A0: Register = Register::GPR4;
    pub const A1: Register = Register::GPR5;
    pub const A2: Register = Register::GPR6;
    pub const A3: Register = Register::GPR7;
    pub const T0: Register = Register::GPR8;
    pub const T1: Register = Register::GPR9;
    pub const T2: Register = Register::GPR10;
    pub const T3: Register = Register::GPR11;
    pub const T4: Register = Register::GPR12;
    pub const T5: Register = Register::GPR13;
    pub const T6: Register = Register::GPR14;
    pub const T7: Register = Register::GPR15;
    pub const S0: Register = Register::GPR16;
    pub const S1: Register = Register::GPR17;
    pub const S2: Register = Register::GPR18;
    pub const S3: Register = Register::GPR19;
    pub const S4: Register = Register::GPR20;
    pub const S5: Register = Register::GPR21;
    pub const S6: Register = Register::GPR22;
    pub const S7: Register = Register::GPR23;
    pub const T8: Register = Register::GPR24;
    pub const T9: Register = Register::GPR25;
    pub const K0: Register = Register::GPR26;
    pub const K1: Register = Register::GPR27;
    pub const GP: Register = Register::GPR28;
    pub const SP: Register = Register::GPR29;
    pub const FP: Register = Register::GPR30;
    pub const S8: Register = Register::GPR30;
    pub const RA: Register = Register::GPR31;
    pub const HI0: Register = Register::AC0;
    pub const HI1: Register = Register::AC1;
    pub const HI2: Register = Register::AC2;
    pub const HI3: Register = Register::AC3;
    pub const LO0: Register = Register::AC0;
    pub const LO1: Register = Register::AC1;
    pub const LO2: Register = Register::AC2;
    pub const LO3: Register = Register::AC3;
}

impl From<Register> for i32 {
    fn from(r: Register) -> Self {
        r as i32
    }
}
