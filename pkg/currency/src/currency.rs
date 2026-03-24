// lint-long-file-override allow-max-lines=1100
#[cfg(feature = "diesel")]
use std::io::Write;

use country::Country;
#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};
use element::Element;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

use crate::error::{Error, Result};

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[allow(clippy::upper_case_acronyms)]
#[derive(
    Debug,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Display,
    EnumString,
)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Text))]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum Currency {
    // Fiat
    AED,
    AFN,
    ALL,
    AMD,
    ANG,
    AOA,
    ARS,
    AUD,
    AWG,
    AZN,
    BAM,
    BBD,
    BDT,
    BGN,
    BHD,
    BIF,
    BMD,
    BND,
    BOB,
    BRL,
    BSD,
    BTN,
    BWP,
    BYN,
    BZD,
    CAD,
    CDF,
    CHF,
    CLP,
    CNY,
    COP,
    CRC,
    CUP,
    CVE,
    CZK,
    DJF,
    DKK,
    DOP,
    DZD,
    EGP,
    ERN,
    ETB,
    EUR,
    FJD,
    FKP,
    GBP,
    GEL,
    GHS,
    GIP,
    GMD,
    GNF,
    GTQ,
    GYD,
    HKD,
    HNL,
    HRK,
    HTG,
    HUF,
    IDR,
    ILS,
    INR,
    IQD,
    IRR,
    ISK,
    JMD,
    JOD,
    JPY,
    KES,
    KGS,
    KHR,
    KMF,
    KPW,
    KRW,
    KWD,
    KYD,
    KZT,
    LAK,
    LBP,
    LKR,
    LRD,
    LSL,
    LYD,
    MAD,
    MDL,
    MGA,
    MKD,
    MMK,
    MNT,
    MOP,
    MRU,
    MUR,
    MVR,
    MWK,
    MXN,
    MYR,
    MZN,
    NAD,
    NGN,
    NIO,
    NOK,
    NPR,
    NZD,
    OMR,
    PAB,
    PEN,
    PGK,
    PHP,
    PKR,
    PLN,
    PYG,
    QAR,
    RON,
    RSD,
    RUB,
    RWF,
    SAR,
    SBD,
    SCR,
    SDG,
    SEK,
    SGD,
    SHP,
    SLL,
    SOS,
    SRD,
    SSP,
    STN,
    SVC,
    SYP,
    SZL,
    THB,
    TJS,
    TMT,
    TND,
    TOP,
    TRY,
    TTD,
    TWD,
    TZS,
    UAH,
    UGX,
    USD,
    UYU,
    UZS,
    VES,
    VND,
    VUV,
    WST,
    XAF,
    XCD,
    XOF,
    XPF,
    YER,
    ZAR,
    ZMW,
    ZWL,

    // Crypto
    USDC,
    ETH,
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for Currency {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for Currency {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized method kind".into())
    }
}

impl Currency {
    pub fn from_str_code(code: &str) -> Result<Self> {
        code.to_ascii_uppercase()
            .parse()
            .map_err(|_| Error::InvalidCurrencyCode)
    }

    pub fn decimals(&self) -> u32 {
        match self {
            // Fiat currencies typically have 2 decimals
            Currency::AED => 2,
            Currency::AFN => 2,
            Currency::ALL => 2,
            Currency::AMD => 2,
            Currency::ANG => 2,
            Currency::AOA => 2,
            Currency::ARS => 2,
            Currency::AUD => 2,
            Currency::AWG => 2,
            Currency::AZN => 2,
            Currency::BAM => 2,
            Currency::BBD => 2,
            Currency::BDT => 2,
            Currency::BGN => 2,
            Currency::BHD => 3, // Bahraini dinar has 3 decimals
            Currency::BIF => 0, // Burundian franc has no decimals
            Currency::BMD => 2,
            Currency::BND => 2,
            Currency::BOB => 2,
            Currency::BRL => 2,
            Currency::BSD => 2,
            Currency::BTN => 2,
            Currency::BWP => 2,
            Currency::BYN => 2,
            Currency::BZD => 2,
            Currency::CAD => 2,
            Currency::CDF => 2,
            Currency::CHF => 2,
            Currency::CLP => 0, // Chilean peso has no decimals
            Currency::CNY => 2,
            Currency::COP => 2,
            Currency::CRC => 2,
            Currency::CUP => 2,
            Currency::CVE => 2,
            Currency::CZK => 2,
            Currency::DJF => 0, // Djiboutian franc has no decimals
            Currency::DKK => 2,
            Currency::DOP => 2,
            Currency::DZD => 2,
            Currency::EGP => 2,
            Currency::ERN => 2,
            Currency::ETB => 2,
            Currency::EUR => 2,
            Currency::FJD => 2,
            Currency::FKP => 2,
            Currency::GBP => 2,
            Currency::GEL => 2,
            Currency::GHS => 2,
            Currency::GIP => 2,
            Currency::GMD => 2,
            Currency::GNF => 0, // Guinean franc has no decimals
            Currency::GTQ => 2,
            Currency::GYD => 2,
            Currency::HKD => 2,
            Currency::HNL => 2,
            Currency::HRK => 2,
            Currency::HTG => 2,
            Currency::HUF => 2,
            Currency::IDR => 2,
            Currency::ILS => 2,
            Currency::INR => 2,
            Currency::IQD => 3, // Iraqi dinar has 3 decimals
            Currency::IRR => 2,
            Currency::ISK => 0, // Icelandic króna has no decimals
            Currency::JMD => 2,
            Currency::JOD => 3, // Jordanian dinar has 3 decimals
            Currency::JPY => 0, // Japanese yen has no decimals
            Currency::KES => 2,
            Currency::KGS => 2,
            Currency::KHR => 2,
            Currency::KMF => 0, // Comorian franc has no decimals
            Currency::KPW => 2,
            Currency::KRW => 0, // South Korean won has no decimals
            Currency::KWD => 3, // Kuwaiti dinar has 3 decimals
            Currency::KYD => 2,
            Currency::KZT => 2,
            Currency::LAK => 2,
            Currency::LBP => 2,
            Currency::LKR => 2,
            Currency::LRD => 2,
            Currency::LSL => 2,
            Currency::LYD => 3, // Libyan dinar has 3 decimals
            Currency::MAD => 2,
            Currency::MDL => 2,
            Currency::MGA => 2,
            Currency::MKD => 2,
            Currency::MMK => 2,
            Currency::MNT => 2,
            Currency::MOP => 2,
            Currency::MRU => 2,
            Currency::MUR => 2,
            Currency::MVR => 2,
            Currency::MWK => 2,
            Currency::MXN => 2,
            Currency::MYR => 2,
            Currency::MZN => 2,
            Currency::NAD => 2,
            Currency::NGN => 2,
            Currency::NIO => 2,
            Currency::NOK => 2,
            Currency::NPR => 2,
            Currency::NZD => 2,
            Currency::OMR => 3, // Omani rial has 3 decimals
            Currency::PAB => 2,
            Currency::PEN => 2,
            Currency::PGK => 2,
            Currency::PHP => 2,
            Currency::PKR => 2,
            Currency::PLN => 2,
            Currency::PYG => 0, // Paraguayan guaraní has no decimals
            Currency::QAR => 2,
            Currency::RON => 2,
            Currency::RSD => 2,
            Currency::RUB => 2,
            Currency::RWF => 0, // Rwandan franc has no decimals
            Currency::SAR => 2,
            Currency::SBD => 2,
            Currency::SCR => 2,
            Currency::SDG => 2,
            Currency::SEK => 2,
            Currency::SGD => 2,
            Currency::SHP => 2,
            Currency::SLL => 2,
            Currency::SOS => 2,
            Currency::SRD => 2,
            Currency::SSP => 2,
            Currency::STN => 2,
            Currency::SVC => 2,
            Currency::SYP => 2,
            Currency::SZL => 2,
            Currency::THB => 2,
            Currency::TJS => 2,
            Currency::TMT => 2,
            Currency::TND => 3, // Tunisian dinar has 3 decimals
            Currency::TOP => 2,
            Currency::TRY => 2,
            Currency::TTD => 2,
            Currency::TWD => 2,
            Currency::TZS => 2,
            Currency::UAH => 2,
            Currency::UGX => 0, // Ugandan shilling has no decimals
            Currency::USD => 2,
            Currency::UYU => 2,
            Currency::UZS => 2,
            Currency::VES => 2,
            Currency::VND => 0, // Vietnamese đồng has no decimals
            Currency::VUV => 0, // Vanuatu vatu has no decimals
            Currency::WST => 2,
            Currency::XAF => 0, // CFA franc BEAC has no decimals
            Currency::XCD => 2,
            Currency::XOF => 0, // CFA franc BCEAO has no decimals
            Currency::XPF => 0, // CFP franc has no decimals
            Currency::YER => 2,
            Currency::ZAR => 2,
            Currency::ZMW => 2,
            Currency::ZWL => 2,

            // Crypto
            Currency::USDC => 6,
            Currency::ETH => 18,
        }
    }

    pub fn to_country(self) -> Country {
        match self {
            Currency::AED => Country::AE,
            Currency::AFN => Country::AF,
            Currency::ALL => Country::AL,
            Currency::AMD => Country::AM,
            Currency::ANG => Country::CW,
            Currency::AOA => Country::AO,
            Currency::ARS => Country::AR,
            Currency::AUD => Country::AU,
            Currency::AWG => Country::AW,
            Currency::AZN => Country::AZ,
            Currency::BAM => Country::BA,
            Currency::BBD => Country::BB,
            Currency::BDT => Country::BD,
            Currency::BGN => Country::BG,
            Currency::BHD => Country::BH,
            Currency::BIF => Country::BI,
            Currency::BMD => Country::BM,
            Currency::BND => Country::BN,
            Currency::BOB => Country::BO,
            Currency::BRL => Country::BR,
            Currency::BSD => Country::BS,
            Currency::BTN => Country::BT,
            Currency::BWP => Country::BW,
            Currency::BYN => Country::BY,
            Currency::BZD => Country::BZ,
            Currency::CAD => Country::CA,
            Currency::CDF => Country::CD,
            Currency::CHF => Country::CH,
            Currency::CLP => Country::CL,
            Currency::CNY => Country::CN,
            Currency::COP => Country::CO,
            Currency::CRC => Country::CR,
            Currency::CUP => Country::CU,
            Currency::CVE => Country::CV,
            Currency::CZK => Country::CZ,
            Currency::DJF => Country::DJ,
            Currency::DKK => Country::DK,
            Currency::DOP => Country::DO,
            Currency::DZD => Country::DZ,
            Currency::EGP => Country::EG,
            Currency::ERN => Country::ER,
            Currency::ETB => Country::ET,
            Currency::EUR => Country::EU,
            Currency::FJD => Country::FJ,
            Currency::FKP => Country::FK,
            Currency::GBP => Country::GB,
            Currency::GEL => Country::GE,
            Currency::GHS => Country::GH,
            Currency::GIP => Country::GI,
            Currency::GMD => Country::GM,
            Currency::GNF => Country::GN,
            Currency::GTQ => Country::GT,
            Currency::GYD => Country::GY,
            Currency::HKD => Country::HK,
            Currency::HNL => Country::HN,
            Currency::HRK => Country::HR,
            Currency::HTG => Country::HT,
            Currency::HUF => Country::HU,
            Currency::IDR => Country::ID,
            Currency::ILS => Country::IL,
            Currency::INR => Country::IN,
            Currency::IQD => Country::IQ,
            Currency::IRR => Country::IR,
            Currency::ISK => Country::IS,
            Currency::JMD => Country::JM,
            Currency::JOD => Country::JO,
            Currency::JPY => Country::JP,
            Currency::KES => Country::KE,
            Currency::KGS => Country::KG,
            Currency::KHR => Country::KH,
            Currency::KMF => Country::KM,
            Currency::KPW => Country::KP,
            Currency::KRW => Country::KR,
            Currency::KWD => Country::KW,
            Currency::KYD => Country::KY,
            Currency::KZT => Country::KZ,
            Currency::LAK => Country::LA,
            Currency::LBP => Country::LB,
            Currency::LKR => Country::LK,
            Currency::LRD => Country::LR,
            Currency::LSL => Country::LS,
            Currency::LYD => Country::LY,
            Currency::MAD => Country::MA,
            Currency::MDL => Country::MD,
            Currency::MGA => Country::MG,
            Currency::MKD => Country::MK,
            Currency::MMK => Country::MM,
            Currency::MNT => Country::MN,
            Currency::MOP => Country::MO,
            Currency::MRU => Country::MR,
            Currency::MUR => Country::MU,
            Currency::MVR => Country::MV,
            Currency::MWK => Country::MW,
            Currency::MXN => Country::MX,
            Currency::MYR => Country::MY,
            Currency::MZN => Country::MZ,
            Currency::NAD => Country::NA,
            Currency::NGN => Country::NG,
            Currency::NIO => Country::NI,
            Currency::NOK => Country::NO,
            Currency::NPR => Country::NP,
            Currency::NZD => Country::NZ,
            Currency::OMR => Country::OM,
            Currency::PAB => Country::PA,
            Currency::PEN => Country::PE,
            Currency::PGK => Country::PG,
            Currency::PHP => Country::PH,
            Currency::PKR => Country::PK,
            Currency::PLN => Country::PL,
            Currency::PYG => Country::PY,
            Currency::QAR => Country::QA,
            Currency::RON => Country::RO,
            Currency::RSD => Country::RS,
            Currency::RUB => Country::RU,
            Currency::RWF => Country::RW,
            Currency::SAR => Country::SA,
            Currency::SBD => Country::SB,
            Currency::SCR => Country::SC,
            Currency::SDG => Country::SD,
            Currency::SEK => Country::SE,
            Currency::SGD => Country::SG,
            Currency::SHP => Country::SH,
            Currency::SLL => Country::SL,
            Currency::SOS => Country::SO,
            Currency::SRD => Country::SR,
            Currency::SSP => Country::SS,
            Currency::STN => Country::ST,
            Currency::SVC => Country::SV,
            Currency::SYP => Country::SY,
            Currency::SZL => Country::SZ,
            Currency::THB => Country::TH,
            Currency::TJS => Country::TJ,
            Currency::TMT => Country::TM,
            Currency::TND => Country::TN,
            Currency::TOP => Country::TO,
            Currency::TRY => Country::TR,
            Currency::TTD => Country::TT,
            Currency::TWD => Country::TW,
            Currency::TZS => Country::TZ,
            Currency::UAH => Country::UA,
            Currency::UGX => Country::UG,
            Currency::USD => Country::US,
            Currency::UYU => Country::UY,
            Currency::UZS => Country::UZ,
            Currency::VES => Country::VE,
            Currency::VND => Country::VN,
            Currency::VUV => Country::VU,
            Currency::WST => Country::WS,
            Currency::XAF => Country::CM, // Central African CFA franc
            Currency::XCD => Country::AG, // East Caribbean dollar
            Currency::XOF => Country::BJ, // West African CFA franc
            Currency::XPF => Country::PF, // CFP franc
            Currency::YER => Country::YE,
            Currency::ZAR => Country::ZA,
            Currency::ZMW => Country::ZM,
            Currency::ZWL => Country::ZW,

            // Crypto
            Currency::USDC => Country::US,
            Currency::ETH => Country::XX,
        }
    }

    pub fn format(&self, value: &Element) -> String {
        let decimal_str = self.to_decimal_string(value);
        let parts: Vec<&str> = decimal_str.split('.').collect();

        // Format integer part with thousand separators
        let int_part = parts[0]
            .chars()
            .rev()
            .collect::<Vec<char>>()
            .chunks(3)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join(",")
            .chars()
            .rev()
            .collect::<String>();

        // Combine with decimal part if it exists and is not all zeros
        let formatted = if parts.len() > 1 {
            let decimal_part = parts[1].trim_end_matches('0');
            if decimal_part.is_empty() {
                int_part
            } else {
                // Ensure at least 2 decimal places are shown
                let min_decimals = if decimal_part.len() == 1 {
                    2
                } else {
                    decimal_part.len()
                };
                let padded_decimal = format!("{decimal_part:0<min_decimals$}");
                format!("{int_part}.{padded_decimal}")
            }
        } else {
            int_part
        };

        self.with_sign(&formatted)
    }

    pub fn with_sign(&self, formatted: &str) -> String {
        match self {
            Currency::AED => format!("د.إ{formatted}"),
            Currency::AFN => format!("؋{formatted}"),
            Currency::ALL => format!("L{formatted}"),
            Currency::AMD => format!("֏{formatted}"),
            Currency::ANG => format!("ƒ{formatted}"),
            Currency::AOA => format!("Kz{formatted}"),
            Currency::ARS => format!("${formatted}"),
            Currency::AUD => format!("A${formatted}"),
            Currency::AWG => format!("ƒ{formatted}"),
            Currency::AZN => format!("₼{formatted}"),
            Currency::BAM => format!("KM{formatted}"),
            Currency::BBD => format!("Bds${formatted}"),
            Currency::BDT => format!("৳{formatted}"),
            Currency::BGN => format!("лв{formatted}"),
            Currency::BHD => format!(".د.ب{formatted}"),
            Currency::BIF => format!("FBu{formatted}"),
            Currency::BMD => format!("BD${formatted}"),
            Currency::BND => format!("B${formatted}"),
            Currency::BOB => format!("Bs{formatted}"),
            Currency::BRL => format!("R${formatted}"),
            Currency::BSD => format!("B${formatted}"),
            Currency::BTN => format!("Nu.{formatted}"),
            Currency::BWP => format!("P{formatted}"),
            Currency::BYN => format!("Br{formatted}"),
            Currency::BZD => format!("BZ${formatted}"),
            Currency::CAD => format!("C${formatted}"),
            Currency::CDF => format!("FC{formatted}"),
            Currency::CHF => format!("CHF{formatted}"),
            Currency::CLP => format!("CLP${formatted}"),
            Currency::CNY => format!("¥{formatted}"),
            Currency::COP => format!("${formatted}"),
            Currency::CRC => format!("₡{formatted}"),
            Currency::CUP => format!("₱{formatted}"),
            Currency::CVE => format!("${formatted}"),
            Currency::CZK => format!("Kč{formatted}"),
            Currency::DJF => format!("Fdj{formatted}"),
            Currency::DKK => format!("kr{formatted}"),
            Currency::DOP => format!("RD${formatted}"),
            Currency::DZD => format!("دج{formatted}"),
            Currency::EGP => format!("E£{formatted}"),
            Currency::ERN => format!("Nfk{formatted}"),
            Currency::ETB => format!("Br{formatted}"),
            Currency::EUR => format!("€{formatted}"),
            Currency::FJD => format!("FJ${formatted}"),
            Currency::FKP => format!("£{formatted}"),
            Currency::GBP => format!("£{formatted}"),
            Currency::GEL => format!("₾{formatted}"),
            Currency::GHS => format!("GH₵{formatted}"),
            Currency::GIP => format!("£{formatted}"),
            Currency::GMD => format!("D{formatted}"),
            Currency::GNF => format!("FG{formatted}"),
            Currency::GTQ => format!("Q{formatted}"),
            Currency::GYD => format!("G${formatted}"),
            Currency::HKD => format!("HK${formatted}"),
            Currency::HNL => format!("L{formatted}"),
            Currency::HRK => format!("kn{formatted}"),
            Currency::HTG => format!("G{formatted}"),
            Currency::HUF => format!("Ft{formatted}"),
            Currency::IDR => format!("Rp{formatted}"),
            Currency::ILS => format!("₪{formatted}"),
            Currency::INR => format!("₹{formatted}"),
            Currency::IQD => format!("ع.د{formatted}"),
            Currency::IRR => format!("﷼{formatted}"),
            Currency::ISK => format!("kr{formatted}"),
            Currency::JMD => format!("J${formatted}"),
            Currency::JOD => format!("د.ا{formatted}"),
            Currency::JPY => format!("¥{formatted}"),
            Currency::KES => format!("KSh{formatted}"),
            Currency::KGS => format!("с{formatted}"),
            Currency::KHR => format!("៛{formatted}"),
            Currency::KMF => format!("CF{formatted}"),
            Currency::KPW => format!("₩{formatted}"),
            Currency::KRW => format!("₩{formatted}"),
            Currency::KWD => format!("د.ك{formatted}"),
            Currency::KYD => format!("CI${formatted}"),
            Currency::KZT => format!("₸{formatted}"),
            Currency::LAK => format!("₭{formatted}"),
            Currency::LBP => format!("L£{formatted}"),
            Currency::LKR => format!("Rs{formatted}"),
            Currency::LRD => format!("L${formatted}"),
            Currency::LSL => format!("L{formatted}"),
            Currency::LYD => format!("ل.د{formatted}"),
            Currency::MAD => format!("د.م.{formatted}"),
            Currency::MDL => format!("L{formatted}"),
            Currency::MGA => format!("Ar{formatted}"),
            Currency::MKD => format!("ден{formatted}"),
            Currency::MMK => format!("K{formatted}"),
            Currency::MNT => format!("₮{formatted}"),
            Currency::MOP => format!("MOP${formatted}"),
            Currency::MRU => format!("UM{formatted}"),
            Currency::MUR => format!("Rs{formatted}"),
            Currency::MVR => format!(".ރ{formatted}"),
            Currency::MWK => format!("MK{formatted}"),
            Currency::MXN => format!("${formatted}"),
            Currency::MYR => format!("RM{formatted}"),
            Currency::MZN => format!("MT{formatted}"),
            Currency::NAD => format!("N${formatted}"),
            Currency::NGN => format!("₦{formatted}"),
            Currency::NIO => format!("C${formatted}"),
            Currency::NOK => format!("kr{formatted}"),
            Currency::NPR => format!("Rs{formatted}"),
            Currency::NZD => format!("NZ${formatted}"),
            Currency::OMR => format!("ر.ع.{formatted}"),
            Currency::PAB => format!("B/{formatted}"),
            Currency::PEN => format!("S/{formatted}"),
            Currency::PGK => format!("K{formatted}"),
            Currency::PHP => format!("₱{formatted}"),
            Currency::PKR => format!("Rs{formatted}"),
            Currency::PLN => format!("zł{formatted}"),
            Currency::PYG => format!("₲{formatted}"),
            Currency::QAR => format!("ر.ق{formatted}"),
            Currency::RON => format!("lei{formatted}"),
            Currency::RSD => format!("дин.{formatted}"),
            Currency::RUB => format!("₽{formatted}"),
            Currency::RWF => format!("FRw{formatted}"),
            Currency::SAR => format!("ر.س{formatted}"),
            Currency::SBD => format!("SI${formatted}"),
            Currency::SCR => format!("SR{formatted}"),
            Currency::SDG => format!("ج.س.{formatted}"),
            Currency::SEK => format!("kr{formatted}"),
            Currency::SGD => format!("S${formatted}"),
            Currency::SHP => format!("£{formatted}"),
            Currency::SLL => format!("Le{formatted}"),
            Currency::SOS => format!("S{formatted}"),
            Currency::SRD => format!("${formatted}"),
            Currency::SSP => format!("£{formatted}"),
            Currency::STN => format!("Db{formatted}"),
            Currency::SVC => format!("₡{formatted}"),
            Currency::SYP => format!("£S{formatted}"),
            Currency::SZL => format!("E{formatted}"),
            Currency::THB => format!("฿{formatted}"),
            Currency::TJS => format!("ЅМ{formatted}"),
            Currency::TMT => format!("m{formatted}"),
            Currency::TND => format!("د.ت{formatted}"),
            Currency::TOP => format!("T${formatted}"),
            Currency::TRY => format!("₺{formatted}"),
            Currency::TTD => format!("TT${formatted}"),
            Currency::TWD => format!("NT${formatted}"),
            Currency::TZS => format!("TSh{formatted}"),
            Currency::UAH => format!("₴{formatted}"),
            Currency::UGX => format!("USh{formatted}"),
            Currency::USD => format!("${formatted}"),
            Currency::UYU => format!("$U{formatted}"),
            Currency::UZS => format!("сўм{formatted}"),
            Currency::VES => format!("Bs.S{formatted}"),
            Currency::VND => format!("₫{formatted}"),
            Currency::VUV => format!("VT{formatted}"),
            Currency::WST => format!("WS${formatted}"),
            Currency::XAF => format!("FCFA{formatted}"),
            Currency::XCD => format!("EC${formatted}"),
            Currency::XOF => format!("CFA{formatted}"),
            Currency::XPF => format!("CFP{formatted}"),
            Currency::YER => format!("﷼{formatted}"),
            Currency::ZAR => format!("R{formatted}"),
            Currency::ZMW => format!("ZK{formatted}"),
            Currency::ZWL => format!("Z${formatted}"),
            Currency::USDC => format!("${formatted}"),
            Currency::ETH => format!("Ξ{formatted}"),
        }
    }

    pub fn resize_decimals(self, value: &Element, to_currency: Currency) -> Element {
        let value = value.to_u256();
        let (base, exp) = if self.decimals() >= to_currency.decimals() {
            // Need to divide to decrease precision
            (value, self.decimals() - to_currency.decimals())
        } else {
            // Need to multiply to increase precision
            (value, to_currency.decimals() - self.decimals())
        };
        let factor = Element::new(10).to_u256().pow(exp);
        let result = if self.decimals() >= to_currency.decimals() {
            base / factor
        } else {
            base * factor
        };
        result.into()
    }

    pub fn to_decimal_string(self, value: &Element) -> String {
        let value = value.to_u256();
        let decimals = self.decimals();
        let factor = Element::new(10).to_u256().pow(decimals);
        let integer_part = value / factor;
        let fractional_part = value % factor;
        format!(
            "{}.{:0width$}",
            integer_part,
            fractional_part,
            width = decimals as usize
        )
    }

    pub fn try_from_decimal_string(&self, value: &str) -> Result<Element> {
        let decimals = self.decimals();
        let parts: Vec<&str> = value.split('.').collect();

        let integer = parts[0]
            .parse::<u64>()
            .map(Element::new)
            .map(|p| p.to_u256())
            .map_err(|_| Error::InvalidDecimalString(value.to_string()))?;

        match parts.len() {
            1 => {
                // No decimal point
                Ok((integer * Element::new(10).to_u256().pow(decimals)).into())
            }
            2 => {
                let mut fractional = parts[1].to_string();

                // Pad or truncate fractional part to match currency decimals
                match fractional.len().cmp(&(decimals as usize)) {
                    std::cmp::Ordering::Less => {
                        fractional.push_str(&"0".repeat(decimals as usize - fractional.len()));
                    }
                    std::cmp::Ordering::Greater => {
                        fractional.truncate(decimals as usize);
                    }
                    std::cmp::Ordering::Equal => {}
                }

                let fractional = fractional
                    .parse::<u64>()
                    .map(Element::new)
                    .map(|p| p.to_u256())
                    .map_err(|_| Error::InvalidDecimalString(value.to_string()))?;
                Ok((integer * Element::new(10).to_u256().pow(decimals) + fractional).into())
            }
            _ => Err(Error::InvalidDecimalString(value.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimals() {
        assert_eq!(Currency::USDC.decimals(), 6);
        assert_eq!(Currency::ARS.decimals(), 2);
        assert_eq!(Currency::MXN.decimals(), 2);
        assert_eq!(Currency::BRL.decimals(), 2);
        assert_eq!(Currency::JPY.decimals(), 0);
    }

    #[test]
    fn test_jpy_format() {
        // JPY has 0 decimals, so the Element value should be the yen amount directly
        assert_eq!(Currency::JPY.format(&Element::new(510)), "¥510");
        assert_eq!(Currency::JPY.format(&Element::new(1_000)), "¥1,000");
        assert_eq!(Currency::JPY.format(&Element::new(51_000)), "¥51,000");
    }

    #[test]
    fn test_kwd_format() {
        // KWD has 3 decimals, test how the formatting actually works
        assert_eq!(Currency::KWD.format(&Element::new(1230)), "د.ك1.23"); // trailing zero trimmed
        assert_eq!(Currency::KWD.format(&Element::new(1234)), "د.ك1.234");
        assert_eq!(Currency::KWD.format(&Element::new(1000)), "د.ك1");
        assert_eq!(Currency::KWD.format(&Element::new(1200)), "د.ك1.20"); // min 2 decimals
    }

    #[test]
    fn test_usdc_format() {
        assert_eq!(Currency::USDC.format(&Element::new(1_234_567)), "$1.234567");
        assert_eq!(Currency::USDC.format(&Element::new(1_230_000)), "$1.23");
        assert_eq!(Currency::USDC.format(&Element::new(1_200_000)), "$1.20");
        assert_eq!(Currency::USDC.format(&Element::new(1_000_000)), "$1");
        assert_eq!(Currency::USDC.format(&Element::new(123)), "$0.000123");
        assert_eq!(Currency::USDC.format(&Element::new(0)), "$0");

        // Test large amounts with thousands separator
        assert_eq!(
            Currency::USDC.format(&Element::new(1_234_567_890_000)),
            "$1,234,567.89"
        );
        assert_eq!(
            Currency::USDC.format(&Element::new(1_000_000_000_000)),
            "$1,000,000"
        );
    }

    #[test]
    fn test_resize_decimals() {
        // 1.234567 USDC -> 1.23 ARS
        assert_eq!(
            Currency::USDC
                .resize_decimals(&Element::new(1_234_567), Currency::ARS)
                .to_u256(),
            Element::new(123).to_u256()
        );

        // 1.000000 USDC -> 1.00 ARS
        assert_eq!(
            Currency::USDC.resize_decimals(&Element::new(1_000_000), Currency::ARS),
            Element::new(100)
        );

        // 0.000123 USDC -> 0.00 ARS (rounds down)
        assert_eq!(
            Currency::USDC.resize_decimals(&Element::new(123), Currency::ARS),
            Element::new(0)
        );

        // 1.23 ARS -> 1.230000 USDC
        assert_eq!(
            Currency::ARS
                .resize_decimals(&Element::new(123), Currency::USDC)
                .to_u256(),
            Element::new(1_230_000).to_u256()
        );

        // 1.00 ARS -> 1.000000 USDC
        assert_eq!(
            Currency::ARS.resize_decimals(&Element::new(100), Currency::USDC),
            Element::new(1_000_000)
        );

        // 0.01 ARS -> 0.010000 USDC
        assert_eq!(
            Currency::ARS.resize_decimals(&Element::new(1), Currency::USDC),
            Element::new(10_000)
        );
    }

    #[test]
    fn test_to_decimal_string() {
        // Test USDC with 6 decimals
        assert_eq!(
            Currency::USDC.to_decimal_string(&Element::new(1_234_567)),
            "1.234567"
        );
        assert_eq!(
            Currency::USDC.to_decimal_string(&Element::new(1_000_000)),
            "1.000000"
        );
        assert_eq!(
            Currency::USDC.to_decimal_string(&Element::new(123)),
            "0.000123"
        );

        // Test fiat currencies with 2 decimals
        assert_eq!(
            Currency::ARS.to_decimal_string(&Element::new(12345)),
            "123.45"
        );
        assert_eq!(
            Currency::MXN.to_decimal_string(&Element::new(10000)),
            "100.00"
        );
        assert_eq!(Currency::BRL.to_decimal_string(&Element::new(99)), "0.99");
    }

    #[test]
    fn test_from_decimal_string() {
        // Test USDC with 6 decimals
        assert_eq!(
            Currency::USDC.try_from_decimal_string("1.234567").unwrap(),
            Element::new(1_234_567)
        );
        assert_eq!(
            Currency::USDC.try_from_decimal_string("1.000000").unwrap(),
            Element::new(1_000_000)
        );
        assert_eq!(
            Currency::USDC.try_from_decimal_string("0.000123").unwrap(),
            Element::new(123)
        );
        assert_eq!(
            Currency::USDC.try_from_decimal_string("1").unwrap(),
            Element::new(1_000_000)
        );
        assert_eq!(
            Currency::USDC.try_from_decimal_string("1.23").unwrap(),
            Element::new(1_230_000)
        );
        assert_eq!(
            Currency::USDC.try_from_decimal_string("1.2345678").unwrap(),
            Element::new(1_234_567)
        ); // Truncates excess decimals

        // Test fiat currencies with 2 decimals
        assert_eq!(
            Currency::ARS.try_from_decimal_string("123.45").unwrap(),
            Element::new(12345)
        );
        assert_eq!(
            Currency::MXN.try_from_decimal_string("100.00").unwrap(),
            Element::new(10000)
        );
        assert_eq!(
            Currency::BRL.try_from_decimal_string("0.99").unwrap(),
            Element::new(99)
        );
        assert_eq!(
            Currency::BRL.try_from_decimal_string("1").unwrap(),
            Element::new(100)
        );
        assert_eq!(
            Currency::BRL.try_from_decimal_string("1.2").unwrap(),
            Element::new(120)
        );
        assert_eq!(
            Currency::BRL.try_from_decimal_string("1.234").unwrap(),
            Element::new(123)
        );
    }
}
