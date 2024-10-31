#![allow(non_camel_case_types)]
use std::ops::Index;
use std::fmt;

use strum::{EnumString, EnumCount, Display, FromRepr};
use strum_macros::EnumIter;

use bitcode;

#[derive(bitcode::Encode, bitcode::Decode, Debug, PartialEq, Eq, Hash, Clone, Copy,
         Display, EnumIter, EnumCount, EnumString, FromRepr)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum Lang {
    abk,
    ace,
    adz,
    afr,
    aii,
    ame,
    amh,
    amr,
    ara,
    arl,
    arn,
    asm,
    ayr,
    aze,
    bak,
    bar,
    bcl,
    bel,
    ben,
    boa,
    bod,
    bpy,
    bre,
    bul,
    cat,
    cbu,
    cdo,
    ceb,
    ces,
    che,
    chk,
    cho,
    chr,
    chv,
    chy,
    ckb,
    cmn,
    cnh,
    cor,
    cos,
    cym,
    dan,
    deu,
    dik,
    diq,
    div,
    ell,
    eng,
    epo,
    est,
    estvro,
    eus,
    ewe,
    ext,
    fao,
    fij,
    fin,
    fini,
    fink,
    finl,
    finm,
    fino,
    finp,
    finr,
    fins,
    fint,
    finx,
    fon,
    fra,
    fry,
    gaz,
    gla,
    gle,
    glg,
    glv,
    gom,
    grn,
    gsw,
    guj,
    hat,
    hbs,
    hbsbos,
    hbshrv,
    hbssrp,
    heb,
    hin,
    hmo,
    hsb,
    hun,
    hus,
    huu,
    hye,
    ibo,
    ido,
    iku,
    ilo,
    ina,
    isl,
    ita,
    izh,
    jpn,
    kac,
    kal,
    kan,
    kat,
    kaz,
    kbd,
    kbp,
    kca,
    khm,
    kir,
    kmr,
    knc,
    koi,
    kor,
    kpv,
    krc,
    ksh,
    lao,
    lat,
    lav,
    lin,
    lit,
    liv,
    lmo,
    ltz,
    lud,
    lug,
    luo,
    lus,
    mal,
    mar,
    mcd,
    mcf,
    mdf,
    mhr,
    mkd,
    mlg,
    mlt,
    mns,
    mon,
    mri,
    mrj,
    msa,
    msaind,
    msamalay,
    msamin,
    msazsm,
    mwl,
    mya,
    myv,
    nav,
    nep,
    nhn,
    nio,
    nld,
    nno,
    nob,
    nso,
    nus,
    oci,
    olo,
    ori,
    oss,
    pag,
    pam,
    pan,
    pbt,
    pes,
    pfl,
    pli,
    pms,
    pnb,
    pol,
    pon,
    por,
    que,
    roh,
    ron,
    rus,
    sag,
    sagb,
    sah,
    sat,
    scn,
    sgs,
    shk,
    shn,
    sin,
    sjd,
    sjk,
    sju,
    slk,
    slv,
    sma,
    sme,
    smj,
    smn,
    sms,
    sna,
    snd,
    som,
    sot,
    spa,
    sqi,
    srd,
    swa,
    swe,
    tam,
    tat,
    tca,
    tel,
    tet,
    tgk,
    tgl,
    tha,
    tir,
    tso,
    tuk,
    tur,
    twi,
    tzh,
    udm,
    uig,
    ukr,
    undhtml,
    und,
    ura,
    urd,
    uzn,
    vie,
    vls,
    vol,
    vot,
    war,
    wln,
    xmf,
    yid,
    yrk,
    yor,
    zul,
}

impl Lang {
    pub fn is_cjk(&self) -> bool {
        *self == Lang::jpn || *self == Lang::kor || *self == Lang::cmn || *self == Lang::cdo
    }

    pub fn collapse(&self) -> Self {
        match self {
            Lang::fini | Lang::fink | Lang::finl | Lang::finm | Lang::fino | Lang::finp | Lang::finr | Lang::fins | Lang::fint | Lang::finx => Lang::fin,
            Lang::hbsbos | Lang::hbshrv | Lang::hbssrp => Lang::hbs,
            Lang::estvro => Lang::est,
            Lang::msaind | Lang::msamalay | Lang::msamin | Lang::msazsm => Lang::msa,
            Lang::sagb => Lang::sag,
            Lang::undhtml => Lang::und,
            _ => self.clone(),
        }
    }
}

/**
 * Simple vector to store scores of each language
 * as fast alternative to a hashmap<lang, f32> if all or almost all languges have to be stored
 * it takes advantage of unkerlying u8 representation of the Lang enum
 */
macro_rules! lang_scores {
($name: ident, $lang: ident, $size: expr) => {
    pub struct $name {
        inner: [f32; $size],
    }

    impl $name {
        pub fn new() -> Self {
            Self { inner: [0.0; $size] }
        }

        pub fn get(&self, lang: $lang) -> f32 {
            self.inner[lang as usize]
        }

        pub fn add_index(&mut self, index: usize, score: f32) {
            self.inner[index] += score;
        }

        pub fn insert(&mut self, lang: $lang, score: f32) {
            self.inner[lang as usize] = score;
        }

        pub fn add(&mut self, other: &Self) {
            for i in 0..$size {
                self.inner[i] += other.inner[i];
            }
        }

        // Normalize scores dividing by a given value
        pub fn norm(&mut self, y: f32) {
            for i in 0..$size {
                self.inner[i] /= y;
            }
        }

        // Reset all values to 0
        pub fn reset(&mut self) {
            for i in 0..$size {
                self.inner[i] = 0.0;
            }
        }
    }

    impl fmt::Debug for $name {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{{")?;
            for (i, val) in self.inner.iter().enumerate() {
                if i != 0 {
                    write!(f," ")?;
                }
                write!(f, "{}={}", $lang::from_repr(i as u8).unwrap(), val)?;
            }
            write!(f, "}}")
        }
    }
};
}

macro_rules! lang_bitmap {
($name: ident, $lang: ident, $size: expr) => {
    pub struct $name {
        inner: [bool; $size],
    }

    impl $name {
        pub fn new() -> Self {
            Self { inner: [false; $size] }
        }

        pub fn get(&self, lang: &$lang) -> bool {
            self.inner[*lang as usize]
        }

        pub fn set(&mut self, lang: &$lang, val: bool) {
            self.inner[*lang as usize] = val;
        }

        // Reset all values to 0
        pub fn reset(&mut self) {
            for i in 0..$size {
                self.inner[i] = false;
            }
        }
    }

    impl Index<usize> for $name {
        type Output = bool;

        fn index(&self, index: usize) -> &bool {
            &self.inner[index]
        }
    }

    impl fmt::Debug for $name {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{{")?;
            for (i, val) in self.inner.iter().enumerate() {
                if i != 0 {
                    write!(f," ")?;
                }
                write!(f, "{}={}", $lang::from_repr(i as u8).unwrap(), val)?;
            }
            write!(f, "}}")
        }
    }
};
}

lang_scores!(LangScores, Lang, Lang::COUNT);
lang_bitmap!(LangBitmap, Lang, Lang::COUNT);
