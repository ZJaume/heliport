use rkyv::{self, Archive, Deserialize, Serialize};
use strum::{EnumString, EnumCount, Display};

use self::Lang::*;
use std::slice::Iter;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Eq, Hash, Clone,
         Display, EnumCount, EnumString)]
#[archive_attr(derive(Debug, PartialEq, Eq, Hash))]
#[repr(u8)]
pub enum Lang {
    Abk, Adz, Afr, Aii, Ame, Amh, Amr, Ara, Arl, Arn, Asm,
    Aze, Bar, Bcl, Bel, Ben, Boa, Bod, Bpy, Bre, Bul, Cat,
    Cbu, Cdo, Ceb, Ces, Che, Chk, Cho, Chr, Chv, Chy, Cmn,
    Cnh, Cor, Cos, Cym, Dan, Deu, Diq, Div, Ell, Eng, Epo,
    Est, Eus, Ewe, Ext, Fao, Fij, Fin, Fini, Fink, Finl, Finm,
    Fino, Finp, Finr, Fins, Fint, Finx, Fra, Fry, Gla, Gle, Glg,
    Glv, Gom, Grn, Gsw, Guj, Hat, Hbs, Hbsbos, Hbshrv, Hbssrp, Heb, Hin,
    Hmo, Hsb, Hun, Hus, Huu, Hye, Ibo, Ido, Iku, Ilo, Ina,
    Isl, Ita, Izh, Jpn, Kal, Kan, Kat, Kaz, Kbd, Kbp, Kca,
    Khm, Kir, Koi, Kor, Kpv, Krc, Ksh, Lao, Lat, Lav, Lin,
    Lit, Liv, Lmo, Ltz, Lud, Lug, Lus, Mal, Mar, Mcd, Mcf,
    Mdf, Mhr, Mkd, Mlg, Mlt, Mns, Mon, Mri, Mrj, Msa, Msaind, Msamin,
    Msazsm, Mwl, Mya, Myv, Nav, Nep, Nhn, Nio, Nld, Nno, Nob,
    Nso, Oci, Olo, Ori, Oss, Pam, Pan, Pes, Pfl, Pli, Pms,
    Pnb, Pol, Pon, Por, Que, Roh, Ron, Rus, Sag, Sah, Scn,
    Sgs, Shk, Sin, Sjd, Sjk, Sju, Slk, Slv, Sma, Sme, Smj,
    Smn, Sms, Sna, Snd, Som, Sot, Spa, Sqi, Srd, Swa, Swe,
    Tam, Tat, Tca, Tel, Tet, Tgk, Tgl, Tha, Tso, Tuk, Tur,
    Tzh, Udm, Uig, Ukr, Ura, Urd, Uzn, Vie, Vls, Vol, Vot,
    Wln, Xmf, Yid, Yrk, Zul,
    Und,
}

impl Lang {
    pub fn is_cjk(&self) -> bool {
        *self == Lang::Jpn || *self == Lang::Kor || *self == Lang::Cmn
    }

    pub fn macrolang(&self) -> Self {
        match self {
            Fini | Fink | Finl | Finm | Fino | Finp | Finr | Fins | Fint | Finx => return Fin,
            _ => self.clone(),
        }
    }

    // iterator over all languages that have language models
    pub fn iter() -> Iter<'static, Lang> {
        static LANGS: [Lang; 214] = [
            Abk, Adz, Afr, Aii, Ame, Amh, Amr, Ara, Arl, Arn, Asm,
            Aze, Bar, Bcl, Bel, Ben, Boa, Bod, Bpy, Bre, Bul, Cat,
            Cbu, Cdo, Ceb, Ces, Che, Chk, Cho, Chr, Chv, Chy, Cmn,
            Cnh, Cor, Cos, Cym, Dan, Deu, Diq, Div, Ell, Eng, Epo,
            Est, Eus, Ewe, Ext, Fao, Fij, Fin, Fini, Fink, Finl, Finm,
            Fino, Finp, Finr, Fins, Fint, Finx, Fra, Fry, Gla, Gle, Glg,
            Glv, Gom, Grn, Gsw, Guj, Hat, Hbsbos, Hbshrv, Hbssrp, Heb, Hin,
            Hmo, Hsb, Hun, Hus, Huu, Hye, Ibo, Ido, Iku, Ilo, Ina,
            Isl, Ita, Izh, Jpn, Kal, Kan, Kat, Kaz, Kbd, Kbp, Kca,
            Khm, Kir, Koi, Kor, Kpv, Krc, Ksh, Lao, Lat, Lav, Lin,
            Lit, Liv, Lmo, Ltz, Lud, Lug, Lus, Mal, Mar, Mcd, Mcf,
            Mdf, Mhr, Mkd, Mlg, Mlt, Mns, Mon, Mri, Mrj, Msaind, Msamin,
            Msazsm, Mwl, Mya, Myv, Nav, Nep, Nhn, Nio, Nld, Nno, Nob,
            Nso, Oci, Olo, Ori, Oss, Pam, Pan, Pes, Pfl, Pli, Pms,
            Pnb, Pol, Pon, Por, Que, Roh, Ron, Rus, Sag, Sah, Scn,
            Sgs, Shk, Sin, Sjd, Sjk, Sju, Slk, Slv, Sma, Sme, Smj,
            Smn, Sms, Sna, Snd, Som, Sot, Spa, Sqi, Srd, Swa, Swe,
            Tam, Tat, Tca, Tel, Tet, Tgk, Tgl, Tha, Tso, Tuk, Tur,
            Tzh, Udm, Uig, Ukr, Ura, Urd, Uzn, Vie, Vls, Vol, Vot,
            Wln, Xmf, Yid, Yrk, Zul,
            ];
        LANGS.iter()
    }

    // iterator adding "und" tag
    pub fn iter_und() -> Iter<'static, Lang> {
        static LANGS: [Lang; 215] = [
            Abk, Adz, Afr, Aii, Ame, Amh, Amr, Ara, Arl, Arn, Asm,
            Aze, Bar, Bcl, Bel, Ben, Boa, Bod, Bpy, Bre, Bul, Cat,
            Cbu, Cdo, Ceb, Ces, Che, Chk, Cho, Chr, Chv, Chy, Cmn,
            Cnh, Cor, Cos, Cym, Dan, Deu, Diq, Div, Ell, Eng, Epo,
            Est, Eus, Ewe, Ext, Fao, Fij, Fin, Fini, Fink, Finl, Finm,
            Fino, Finp, Finr, Fins, Fint, Finx, Fra, Fry, Gla, Gle, Glg,
            Glv, Gom, Grn, Gsw, Guj, Hat, Hbsbos, Hbshrv, Hbssrp, Heb, Hin,
            Hmo, Hsb, Hun, Hus, Huu, Hye, Ibo, Ido, Iku, Ilo, Ina,
            Isl, Ita, Izh, Jpn, Kal, Kan, Kat, Kaz, Kbd, Kbp, Kca,
            Khm, Kir, Koi, Kor, Kpv, Krc, Ksh, Lao, Lat, Lav, Lin,
            Lit, Liv, Lmo, Ltz, Lud, Lug, Lus, Mal, Mar, Mcd, Mcf,
            Mdf, Mhr, Mkd, Mlg, Mlt, Mns, Mon, Mri, Mrj, Msaind, Msamin,
            Msazsm, Mwl, Mya, Myv, Nav, Nep, Nhn, Nio, Nld, Nno, Nob,
            Nso, Oci, Olo, Ori, Oss, Pam, Pan, Pes, Pfl, Pli, Pms,
            Pnb, Pol, Pon, Por, Que, Roh, Ron, Rus, Sag, Sah, Scn,
            Sgs, Shk, Sin, Sjd, Sjk, Sju, Slk, Slv, Sma, Sme, Smj,
            Smn, Sms, Sna, Snd, Som, Sot, Spa, Sqi, Srd, Swa, Swe,
            Tam, Tat, Tca, Tel, Tet, Tgk, Tgl, Tha, Tso, Tuk, Tur,
            Tzh, Udm, Uig, Ukr, Ura, Urd, Uzn, Vie, Vls, Vol, Vot,
            Wln, Xmf, Yid, Yrk, Zul,
            Und,
            ];
        LANGS.iter()
    }

}

// impl FromStr for Lang {
//     type Err = ();

//     fn from_str(input: &str) -> Result<Lang, Self::Err> {
//         match input {
//         }
//     }
// }
