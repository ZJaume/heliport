#![allow(non_camel_case_types)]
use std::fmt;

use strum::{EnumString, EnumCount, Display, FromRepr};
use strum_macros::EnumIter;

use bitcode;

#[derive(bitcode::Encode, bitcode::Decode, Debug, PartialEq, Eq, Hash, Clone, Copy,
         Display, EnumIter, EnumCount, EnumString, FromRepr)]
#[strum(serialize_all = "lowercase")]
#[repr(u8)]
pub enum Lang {
    ace_Arab, ace_Latn, afr_Latn, als_Latn, amh_Ethi, ara_Arab, asm_Beng,
    ast_Latn, awa_Deva, ayr_Latn, azb_Arab, azj_Latn, bak_Cyrl, bam_Latn,
    ban_Latn, bel_Cyrl, bem_Latn, ben_Beng, bho_Deva, bjn_Arab, bjn_Latn,
    bod_Tibt, bos_Latn, bug_Latn, bul_Cyrl, cat_Latn, ceb_Latn, ces_Latn,
    cjk_Latn, ckb_Arab, crh_Latn, cym_Latn, dan_Latn, deu_Latn, dik_Latn,
    dyu_Latn, dzo_Tibt, ell_Grek, eng_Latn, epo_Latn, est_Latn, eus_Latn,
    ewe_Latn, fao_Latn, fij_Latn, fin_Latn, fon_Latn, fra_Latn, fur_Latn,
    fuv_Latn, gaz_Latn, gla_Latn, gle_Latn, glg_Latn, grn_Latn, guj_Gujr,
    hat_Latn, hau_Latn, heb_Hebr, hin_Deva, hne_Deva, hrv_Latn, hun_Latn,
    hye_Armn, ibo_Latn, ilo_Latn, ind_Latn, isl_Latn, ita_Latn, jav_Latn,
    jpn_Jpan, kab_Latn, kac_Latn, kam_Latn, kan_Knda, kas_Arab, kas_Deva,
    kat_Geor, kaz_Cyrl, kbp_Latn, kea_Latn, khk_Cyrl, khm_Khmr, kik_Latn,
    kin_Latn, kir_Cyrl, kmb_Latn, kmr_Latn, knc_Arab, knc_Latn, kon_Latn,
    kor_Hang, lao_Laoo, lij_Latn, lim_Latn, lin_Latn, lit_Latn, lmo_Latn,
    ltg_Latn, ltz_Latn, lua_Latn, lug_Latn, luo_Latn, lus_Latn, lvs_Latn,
    mag_Deva, mai_Deva, mal_Mlym, mar_Deva, min_Latn, mkd_Cyrl, mlt_Latn,
    mni_Beng, mos_Latn, mri_Latn, mya_Mymr, nld_Latn, nno_Latn, nob_Latn,
    npi_Deva, nso_Latn, nus_Latn, nya_Latn, oci_Latn, ory_Orya, pag_Latn,
    pan_Guru, pap_Latn, pbt_Arab, pes_Arab, plt_Latn, pol_Latn, por_Latn,
    prs_Arab, quy_Latn, ron_Latn, run_Latn, rus_Cyrl, sag_Latn, san_Deva,
    sat_Olck, scn_Latn, shn_Mymr, sin_Sinh, slk_Latn, slv_Latn, smo_Latn,
    sna_Latn, snd_Arab, som_Latn, sot_Latn, spa_Latn, srd_Latn, srp_Cyrl,
    ssw_Latn, sun_Latn, swe_Latn, swh_Latn, szl_Latn, tam_Taml, taq_Latn,
    taq_Tfng, tat_Cyrl, tel_Telu, tgk_Cyrl, tgl_Latn, tha_Thai, tir_Ethi,
    tpi_Latn, tsn_Latn, tso_Latn, tuk_Latn, tum_Latn, tur_Latn, twi_Latn,
    tzm_Tfng, uig_Arab, ukr_Cyrl, umb_Latn, urd_Arab, uzn_Latn, vec_Latn,
    vie_Latn, war_Latn, wol_Latn, xho_Latn, ydd_Hebr, yor_Latn, yue_Hant,
    zho_Hans, zho_Hant, zsm_Latn, zul_Latn, unk,
}

impl Lang {
    pub fn is_cjk(&self) -> bool {
        *self == Lang::jpn_Jpan || *self == Lang::kor_Hang || *self == Lang::zho_Hans
           || *self == Lang::zho_Hant || *self == Lang::yue_Hant
    }

    pub fn macrolang(&self) -> Self {
        match self {
            // Fini | Fink | Finl | Finm | Fino | Finp | Finr | Fins | Fint | Finx => return Fin,
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

lang_scores!(LangScores, Lang, Lang::COUNT);
