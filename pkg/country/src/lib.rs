// lint-long-file-override allow-max-lines=1100
#[cfg(feature = "diesel")]
use std::io::Write;

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[cfg(feature = "diesel")]
use diesel::{
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
};

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
pub enum Country {
    #[serde(alias = "AFG")]
    AF,
    #[serde(alias = "ALA")]
    AX,
    #[serde(alias = "ALB")]
    AL,
    #[serde(alias = "DZA")]
    DZ,
    #[serde(alias = "ASM")]
    AS,
    #[serde(alias = "AND")]
    AD,
    #[serde(alias = "AGO")]
    AO,
    #[serde(alias = "AIA")]
    AI,
    #[serde(alias = "ATA")]
    AQ,
    #[serde(alias = "ATG")]
    AG,
    #[serde(alias = "ARG")]
    AR,
    #[serde(alias = "ARM")]
    AM,
    #[serde(alias = "ABW")]
    AW,
    #[serde(alias = "AUS")]
    AU,
    #[serde(alias = "AUT")]
    AT,
    #[serde(alias = "AZE")]
    AZ,
    #[serde(alias = "BHS")]
    BS,
    #[serde(alias = "BHR")]
    BH,
    #[serde(alias = "BGD")]
    BD,
    #[serde(alias = "BRB")]
    BB,
    #[serde(alias = "BLR")]
    BY,
    #[serde(alias = "BEL")]
    BE,
    #[serde(alias = "BLZ")]
    BZ,
    #[serde(alias = "BEN")]
    BJ,
    #[serde(alias = "BMU")]
    BM,
    #[serde(alias = "BTN")]
    BT,
    #[serde(alias = "BOL")]
    BO,
    #[serde(alias = "BIH")]
    BA,
    #[serde(alias = "BWA")]
    BW,
    #[serde(alias = "BVT")]
    BV,
    #[serde(alias = "BRA")]
    BR,
    #[serde(alias = "IOT")]
    IO,
    #[serde(alias = "VGB")]
    VG,
    #[serde(alias = "BRN")]
    BN,
    #[serde(alias = "BGR")]
    BG,
    #[serde(alias = "BFA")]
    BF,
    #[serde(alias = "BDI")]
    BI,
    #[serde(alias = "KHM")]
    KH,
    #[serde(alias = "CMR")]
    CM,
    #[serde(alias = "CAN")]
    CA,
    #[serde(alias = "CPV")]
    CV,
    #[serde(alias = "BES")]
    BQ,
    #[serde(alias = "CYM")]
    KY,
    #[serde(alias = "CAF")]
    CF,
    #[serde(alias = "TCD")]
    TD,
    #[serde(alias = "CHL")]
    CL,
    #[serde(alias = "CHN")]
    CN,
    #[serde(alias = "CXR")]
    CX,
    #[serde(alias = "CCK")]
    CC,
    #[serde(alias = "COL")]
    CO,
    #[serde(alias = "COM")]
    KM,
    #[serde(alias = "COG")]
    CG,
    #[serde(alias = "COD")]
    CD,
    #[serde(alias = "COK")]
    CK,
    #[serde(alias = "CRI")]
    CR,
    #[serde(alias = "CIV")]
    CI,
    #[serde(alias = "HRV")]
    HR,
    #[serde(alias = "CUB")]
    CU,
    #[serde(alias = "CUW")]
    CW,
    #[serde(alias = "CYP")]
    CY,
    #[serde(alias = "CZE")]
    CZ,
    #[serde(alias = "DNK")]
    DK,
    #[serde(alias = "DJI")]
    DJ,
    #[serde(alias = "DMA")]
    DM,
    #[serde(alias = "DOM")]
    DO,
    #[serde(alias = "ECU")]
    EC,
    #[serde(alias = "EGY")]
    EG,
    #[serde(alias = "SLV")]
    SV,
    #[serde(alias = "GNQ")]
    GQ,
    #[serde(alias = "ERI")]
    ER,
    #[serde(alias = "EST")]
    EE,
    #[serde(alias = "SWZ")]
    SZ,
    #[serde(alias = "ETH")]
    ET,
    #[serde(alias = "FLK")]
    FK,
    #[serde(alias = "FRO")]
    FO,
    #[serde(alias = "FJI")]
    FJ,
    #[serde(alias = "FIN")]
    FI,
    #[serde(alias = "FRA")]
    FR,
    #[serde(alias = "GUF")]
    GF,
    #[serde(alias = "PYF")]
    PF,
    #[serde(alias = "ATF")]
    TF,
    #[serde(alias = "GAB")]
    GA,
    #[serde(alias = "GMB")]
    GM,
    #[serde(alias = "GEO")]
    GE,
    #[serde(alias = "DEU")]
    DE,
    #[serde(alias = "GHA")]
    GH,
    #[serde(alias = "GIB")]
    GI,
    #[serde(alias = "GRC")]
    GR,
    #[serde(alias = "GRL")]
    GL,
    #[serde(alias = "GRD")]
    GD,
    #[serde(alias = "GLP")]
    GP,
    #[serde(alias = "GUM")]
    GU,
    #[serde(alias = "GTM")]
    GT,
    #[serde(alias = "GGY")]
    GG,
    #[serde(alias = "GIN")]
    GN,
    #[serde(alias = "GNB")]
    GW,
    #[serde(alias = "GUY")]
    GY,
    #[serde(alias = "HTI")]
    HT,
    #[serde(alias = "HMD")]
    HM,
    #[serde(alias = "HND")]
    HN,
    #[serde(alias = "HKG")]
    HK,
    #[serde(alias = "HUN")]
    HU,
    #[serde(alias = "ISL")]
    IS,
    #[serde(alias = "IND")]
    IN,
    #[serde(alias = "IDN")]
    ID,
    #[serde(alias = "IRN")]
    IR,
    #[serde(alias = "IRQ")]
    IQ,
    #[serde(alias = "IRL")]
    IE,
    #[serde(alias = "IMN")]
    IM,
    #[serde(alias = "ISR")]
    IL,
    #[serde(alias = "ITA")]
    IT,
    #[serde(alias = "JAM")]
    JM,
    #[serde(alias = "JPN")]
    JP,
    #[serde(alias = "JEY")]
    JE,
    #[serde(alias = "JOR")]
    JO,
    #[serde(alias = "KAZ")]
    KZ,
    #[serde(alias = "KEN")]
    KE,
    #[serde(alias = "KIR")]
    KI,
    #[serde(alias = "KWT")]
    KW,
    #[serde(alias = "KGZ")]
    KG,
    #[serde(alias = "LAO")]
    LA,
    #[serde(alias = "LVA")]
    LV,
    #[serde(alias = "LBN")]
    LB,
    #[serde(alias = "LSO")]
    LS,
    #[serde(alias = "LBR")]
    LR,
    #[serde(alias = "LBY")]
    LY,
    #[serde(alias = "LIE")]
    LI,
    #[serde(alias = "LTU")]
    LT,
    #[serde(alias = "LUX")]
    LU,
    #[serde(alias = "MAC")]
    MO,
    #[serde(alias = "MDG")]
    MG,
    #[serde(alias = "MWI")]
    MW,
    #[serde(alias = "MYS")]
    MY,
    #[serde(alias = "MDV")]
    MV,
    #[serde(alias = "MLI")]
    ML,
    #[serde(alias = "MLT")]
    MT,
    #[serde(alias = "MHL")]
    MH,
    #[serde(alias = "MTQ")]
    MQ,
    #[serde(alias = "MRT")]
    MR,
    #[serde(alias = "MUS")]
    MU,
    #[serde(alias = "MYT")]
    YT,
    #[serde(alias = "MEX")]
    MX,
    #[serde(alias = "FSM")]
    FM,
    #[serde(alias = "MDA")]
    MD,
    #[serde(alias = "MCO")]
    MC,
    #[serde(alias = "MNG")]
    MN,
    #[serde(alias = "MNE")]
    ME,
    #[serde(alias = "MSR")]
    MS,
    #[serde(alias = "MAR")]
    MA,
    #[serde(alias = "MOZ")]
    MZ,
    #[serde(alias = "MMR")]
    MM,
    #[serde(alias = "NAM")]
    NA,
    #[serde(alias = "NRU")]
    NR,
    #[serde(alias = "NPL")]
    NP,
    #[serde(alias = "NLD")]
    NL,
    #[serde(alias = "NCL")]
    NC,
    #[serde(alias = "NZL")]
    NZ,
    #[serde(alias = "NIC")]
    NI,
    #[serde(alias = "NER")]
    NE,
    #[serde(alias = "NGA")]
    NG,
    #[serde(alias = "NIU")]
    NU,
    #[serde(alias = "NFK")]
    NF,
    #[serde(alias = "PRK")]
    KP,
    #[serde(alias = "MKD")]
    MK,
    #[serde(alias = "MNP")]
    MP,
    #[serde(alias = "NOR")]
    NO,
    #[serde(alias = "OMN")]
    OM,
    #[serde(alias = "PAK")]
    PK,
    #[serde(alias = "PLW")]
    PW,
    #[serde(alias = "PSE")]
    PS,
    #[serde(alias = "PAN")]
    PA,
    #[serde(alias = "PNG")]
    PG,
    #[serde(alias = "PRY")]
    PY,
    #[serde(alias = "PER")]
    PE,
    #[serde(alias = "PHL")]
    PH,
    #[serde(alias = "PCN")]
    PN,
    #[serde(alias = "POL")]
    PL,
    #[serde(alias = "PRT")]
    PT,
    #[serde(alias = "PRI")]
    PR,
    #[serde(alias = "QAT")]
    QA,
    #[serde(alias = "REU")]
    RE,
    #[serde(alias = "ROU")]
    RO,
    #[serde(alias = "RUS")]
    RU,
    #[serde(alias = "RWA")]
    RW,
    #[serde(alias = "WSM")]
    WS,
    #[serde(alias = "SMR")]
    SM,
    #[serde(alias = "STP")]
    ST,
    #[serde(alias = "SAU")]
    SA,
    #[serde(alias = "SEN")]
    SN,
    #[serde(alias = "SRB")]
    RS,
    #[serde(alias = "SYC")]
    SC,
    #[serde(alias = "SLE")]
    SL,
    #[serde(alias = "SGP")]
    SG,
    #[serde(alias = "SXM")]
    SX,
    #[serde(alias = "SVK")]
    SK,
    #[serde(alias = "SVN")]
    SI,
    #[serde(alias = "SLB")]
    SB,
    #[serde(alias = "SOM")]
    SO,
    #[serde(alias = "ZAF")]
    ZA,
    #[serde(alias = "SGS")]
    GS,
    #[serde(alias = "KOR")]
    KR,
    #[serde(alias = "SSD")]
    SS,
    #[serde(alias = "ESP")]
    ES,
    #[serde(alias = "LKA")]
    LK,
    #[serde(alias = "BLM")]
    BL,
    #[serde(alias = "SHN")]
    SH,
    #[serde(alias = "KNA")]
    KN,
    #[serde(alias = "LCA")]
    LC,
    #[serde(alias = "MAF")]
    MF,
    #[serde(alias = "SPM")]
    PM,
    #[serde(alias = "VCT")]
    VC,
    #[serde(alias = "SDN")]
    SD,
    #[serde(alias = "SUR")]
    SR,
    #[serde(alias = "SJM")]
    SJ,
    #[serde(alias = "SWE")]
    SE,
    #[serde(alias = "CHE")]
    CH,
    #[serde(alias = "SYR")]
    SY,
    #[serde(alias = "TWN")]
    TW,
    #[serde(alias = "TJK")]
    TJ,
    #[serde(alias = "TZA")]
    TZ,
    #[serde(alias = "THA")]
    TH,
    #[serde(alias = "TLS")]
    TL,
    #[serde(alias = "TGO")]
    TG,
    #[serde(alias = "TKL")]
    TK,
    #[serde(alias = "TON")]
    TO,
    #[serde(alias = "TTO")]
    TT,
    #[serde(alias = "TUN")]
    TN,
    #[serde(alias = "TUR")]
    TR,
    #[serde(alias = "TKM")]
    TM,
    #[serde(alias = "TCA")]
    TC,
    #[serde(alias = "TUV")]
    TV,
    #[serde(alias = "UMI")]
    UM,
    #[serde(alias = "VIR")]
    VI,
    #[serde(alias = "UGA")]
    UG,
    #[serde(alias = "UKR")]
    UA,
    #[serde(alias = "ARE")]
    AE,
    #[serde(alias = "GBR")]
    GB,
    #[serde(alias = "USA")]
    US,
    #[serde(alias = "URY")]
    UY,
    #[serde(alias = "UZB")]
    UZ,
    #[serde(alias = "VUT")]
    VU,
    #[serde(alias = "VAT")]
    VA,
    #[serde(alias = "VEN")]
    VE,
    #[serde(alias = "VNM")]
    VN,
    #[serde(alias = "WLF")]
    WF,
    #[serde(alias = "ESH")]
    EH,
    #[serde(alias = "YEM")]
    YE,
    #[serde(alias = "ZMB")]
    ZM,
    #[serde(alias = "ZWE")]
    ZW,

    // Abnormal
    #[serde(alias = "EU")]
    EU,
    #[serde(alias = "XX")]
    XX,
}

impl Country {
    pub fn to_country_name(self) -> &'static str {
        match self {
            Country::AF => "Afghanistan",
            Country::AX => "Åland Islands",
            Country::AL => "Albania",
            Country::DZ => "Algeria",
            Country::AS => "American Samoa",
            Country::AD => "Andorra",
            Country::AO => "Angola",
            Country::AI => "Anguilla",
            Country::AQ => "Antarctica",
            Country::AG => "Antigua and Barbuda",
            Country::AR => "Argentina",
            Country::AM => "Armenia",
            Country::AW => "Aruba",
            Country::AU => "Australia",
            Country::AT => "Austria",
            Country::AZ => "Azerbaijan",
            Country::BS => "Bahamas",
            Country::BH => "Bahrain",
            Country::BD => "Bangladesh",
            Country::BB => "Barbados",
            Country::BY => "Belarus",
            Country::BE => "Belgium",
            Country::BZ => "Belize",
            Country::BJ => "Benin",
            Country::BM => "Bermuda",
            Country::BT => "Bhutan",
            Country::BO => "Bolivia",
            Country::BQ => "Bonaire, Sint Eustatius and Saba",
            Country::BA => "Bosnia and Herzegovina",
            Country::BW => "Botswana",
            Country::BV => "Bouvet Island",
            Country::BR => "Brazil",
            Country::IO => "British Indian Ocean Territory",
            Country::BN => "Brunei Darussalam",
            Country::BG => "Bulgaria",
            Country::BF => "Burkina Faso",
            Country::BI => "Burundi",
            Country::KH => "Cambodia",
            Country::CM => "Cameroon",
            Country::CA => "Canada",
            Country::CV => "Cape Verde",
            Country::KY => "Cayman Islands",
            Country::CF => "Central African Republic",
            Country::TD => "Chad",
            Country::CL => "Chile",
            Country::CN => "China",
            Country::CX => "Christmas Island",
            Country::CC => "Cocos (Keeling) Islands",
            Country::CO => "Colombia",
            Country::KM => "Comoros",
            Country::CG => "Congo",
            Country::CD => "Congo, Democratic Republic of the",
            Country::CK => "Cook Islands",
            Country::CR => "Costa Rica",
            Country::CI => "Côte d'Ivoire",
            Country::HR => "Croatia",
            Country::CU => "Cuba",
            Country::CW => "Curaçao",
            Country::CY => "Cyprus",
            Country::CZ => "Czech Republic",
            Country::DK => "Denmark",
            Country::DJ => "Djibouti",
            Country::DM => "Dominica",
            Country::DO => "Dominican Republic",
            Country::EC => "Ecuador",
            Country::EG => "Egypt",
            Country::SV => "El Salvador",
            Country::GQ => "Equatorial Guinea",
            Country::ER => "Eritrea",
            Country::EE => "Estonia",
            Country::ET => "Ethiopia",
            Country::FK => "Falkland Islands (Malvinas)",
            Country::FO => "Faroe Islands",
            Country::FJ => "Fiji",
            Country::FI => "Finland",
            Country::FR => "France",
            Country::GF => "French Guiana",
            Country::PF => "French Polynesia",
            Country::TF => "French Southern Territories",
            Country::GA => "Gabon",
            Country::GM => "Gambia",
            Country::GE => "Georgia",
            Country::DE => "Germany",
            Country::GH => "Ghana",
            Country::GI => "Gibraltar",
            Country::GR => "Greece",
            Country::GL => "Greenland",
            Country::GD => "Grenada",
            Country::GP => "Guadeloupe",
            Country::GU => "Guam",
            Country::GT => "Guatemala",
            Country::GG => "Guernsey",
            Country::GN => "Guinea",
            Country::GW => "Guinea-Bissau",
            Country::GY => "Guyana",
            Country::HT => "Haiti",
            Country::HM => "Heard Island and McDonald Islands",
            Country::VA => "Holy See (Vatican City State)",
            Country::HN => "Honduras",
            Country::HK => "Hong Kong",
            Country::HU => "Hungary",
            Country::IS => "Iceland",
            Country::IN => "India",
            Country::ID => "Indonesia",
            Country::IR => "Iran",
            Country::IQ => "Iraq",
            Country::IE => "Ireland",
            Country::IM => "Isle of Man",
            Country::IL => "Israel",
            Country::IT => "Italy",
            Country::JM => "Jamaica",
            Country::JP => "Japan",
            Country::JE => "Jersey",
            Country::JO => "Jordan",
            Country::KZ => "Kazakhstan",
            Country::KE => "Kenya",
            Country::KI => "Kiribati",
            Country::KP => "Korea, Democratic People's Republic of",
            Country::KR => "Korea, Republic of",
            Country::KW => "Kuwait",
            Country::KG => "Kyrgyzstan",
            Country::LA => "Lao People's Democratic Republic",
            Country::LV => "Latvia",
            Country::LB => "Lebanon",
            Country::LS => "Lesotho",
            Country::LR => "Liberia",
            Country::LY => "Libya",
            Country::LI => "Liechtenstein",
            Country::LT => "Lithuania",
            Country::LU => "Luxembourg",
            Country::MO => "Macao",
            Country::MK => "Macedonia",
            Country::MG => "Madagascar",
            Country::MW => "Malawi",
            Country::MY => "Malaysia",
            Country::MV => "Maldives",
            Country::ML => "Mali",
            Country::MT => "Malta",
            Country::MH => "Marshall Islands",
            Country::MQ => "Martinique",
            Country::MR => "Mauritania",
            Country::MU => "Mauritius",
            Country::YT => "Mayotte",
            Country::MX => "Mexico",
            Country::FM => "Micronesia",
            Country::MD => "Moldova",
            Country::MC => "Monaco",
            Country::MN => "Mongolia",
            Country::ME => "Montenegro",
            Country::MS => "Montserrat",
            Country::MA => "Morocco",
            Country::MZ => "Mozambique",
            Country::MM => "Myanmar",
            Country::NA => "Namibia",
            Country::NR => "Nauru",
            Country::NP => "Nepal",
            Country::NL => "Netherlands",
            Country::NC => "New Caledonia",
            Country::NZ => "New Zealand",
            Country::NI => "Nicaragua",
            Country::NE => "Niger",
            Country::NG => "Nigeria",
            Country::NU => "Niue",
            Country::NF => "Norfolk Island",
            Country::MP => "Northern Mariana Islands",
            Country::NO => "Norway",
            Country::OM => "Oman",
            Country::PK => "Pakistan",
            Country::PW => "Palau",
            Country::PS => "Palestine",
            Country::PA => "Panama",
            Country::PG => "Papua New Guinea",
            Country::PY => "Paraguay",
            Country::PE => "Peru",
            Country::PH => "Philippines",
            Country::PN => "Pitcairn",
            Country::PL => "Poland",
            Country::PT => "Portugal",
            Country::PR => "Puerto Rico",
            Country::QA => "Qatar",
            Country::RE => "Réunion",
            Country::RO => "Romania",
            Country::RU => "Russian Federation",
            Country::RW => "Rwanda",
            Country::BL => "Saint Barthélemy",
            Country::SH => "Saint Helena",
            Country::KN => "Saint Kitts and Nevis",
            Country::LC => "Saint Lucia",
            Country::MF => "Saint Martin (French part)",
            Country::PM => "Saint Pierre and Miquelon",
            Country::VC => "Saint Vincent and the Grenadines",
            Country::WS => "Samoa",
            Country::SM => "San Marino",
            Country::ST => "Sao Tome and Principe",
            Country::SA => "Saudi Arabia",
            Country::SN => "Senegal",
            Country::RS => "Serbia",
            Country::SC => "Seychelles",
            Country::SL => "Sierra Leone",
            Country::SG => "Singapore",
            Country::SX => "Sint Maarten (Dutch part)",
            Country::SK => "Slovakia",
            Country::SI => "Slovenia",
            Country::SB => "Solomon Islands",
            Country::SO => "Somalia",
            Country::ZA => "South Africa",
            Country::GS => "South Georgia and the South Sandwich Islands",
            Country::SS => "South Sudan",
            Country::ES => "Spain",
            Country::LK => "Sri Lanka",
            Country::SD => "Sudan",
            Country::SR => "Suriname",
            Country::SJ => "Svalbard and Jan Mayen",
            Country::SZ => "Swaziland",
            Country::SE => "Sweden",
            Country::CH => "Switzerland",
            Country::SY => "Syrian Arab Republic",
            Country::TW => "Taiwan",
            Country::TJ => "Tajikistan",
            Country::TZ => "Tanzania",
            Country::TH => "Thailand",
            Country::TL => "Timor-Leste",
            Country::TG => "Togo",
            Country::TK => "Tokelau",
            Country::TO => "Tonga",
            Country::TT => "Trinidad and Tobago",
            Country::TN => "Tunisia",
            Country::TR => "Turkey",
            Country::TM => "Turkmenistan",
            Country::TC => "Turks and Caicos Islands",
            Country::TV => "Tuvalu",
            Country::UG => "Uganda",
            Country::UA => "Ukraine",
            Country::AE => "United Arab Emirates",
            Country::GB => "United Kingdom",
            Country::US => "United States",
            Country::UM => "United States Minor Outlying Islands",
            Country::UY => "Uruguay",
            Country::UZ => "Uzbekistan",
            Country::VU => "Vanuatu",
            Country::VE => "Venezuela",
            Country::VN => "Vietnam",
            Country::VG => "Virgin Islands, British",
            Country::VI => "Virgin Islands, U.S.",
            Country::WF => "Wallis and Futuna",
            Country::EH => "Western Sahara",
            Country::YE => "Yemen",
            Country::ZM => "Zambia",
            Country::ZW => "Zimbabwe",

            // Abnormal
            Country::EU => "European Union",
            Country::XX => "Global",
        }
    }
    pub fn three_letter_name(self) -> &'static str {
        match self {
            Country::AF => "AFG",
            Country::AX => "ALA",
            Country::AL => "ALB",
            Country::DZ => "DZA",
            Country::AS => "ASM",
            Country::AD => "AND",
            Country::AO => "AGO",
            Country::AI => "AIA",
            Country::AQ => "ATA",
            Country::AG => "ATG",
            Country::AR => "ARG",
            Country::AM => "ARM",
            Country::AW => "ABW",
            Country::AU => "AUS",
            Country::AT => "AUT",
            Country::AZ => "AZE",
            Country::BS => "BHS",
            Country::BH => "BHR",
            Country::BD => "BGD",
            Country::BB => "BRB",
            Country::BY => "BLR",
            Country::BE => "BEL",
            Country::BZ => "BLZ",
            Country::BJ => "BEN",
            Country::BM => "BMU",
            Country::BT => "BTN",
            Country::BO => "BOL",
            Country::BA => "BIH",
            Country::BW => "BWA",
            Country::BV => "BVT",
            Country::BR => "BRA",
            Country::IO => "IOT",
            Country::VG => "VGB",
            Country::BN => "BRN",
            Country::BG => "BGR",
            Country::BF => "BFA",
            Country::BI => "BDI",
            Country::KH => "KHM",
            Country::CM => "CMR",
            Country::CA => "CAN",
            Country::CV => "CPV",
            Country::BQ => "BES",
            Country::KY => "CYM",
            Country::CF => "CAF",
            Country::TD => "TCD",
            Country::CL => "CHL",
            Country::CN => "CHN",
            Country::CX => "CXR",
            Country::CC => "CCK",
            Country::CO => "COL",
            Country::KM => "COM",
            Country::CG => "COG",
            Country::CD => "COD",
            Country::CK => "COK",
            Country::CR => "CRI",
            Country::CI => "CIV",
            Country::HR => "HRV",
            Country::CU => "CUB",
            Country::CW => "CUW",
            Country::CY => "CYP",
            Country::CZ => "CZE",
            Country::DK => "DNK",
            Country::DJ => "DJI",
            Country::DM => "DMA",
            Country::DO => "DOM",
            Country::EC => "ECU",
            Country::EG => "EGY",
            Country::SV => "SLV",
            Country::GQ => "GNQ",
            Country::ER => "ERI",
            Country::EE => "EST",
            Country::SZ => "SWZ",
            Country::ET => "ETH",
            Country::FK => "FLK",
            Country::FO => "FRO",
            Country::FJ => "FJI",
            Country::FI => "FIN",
            Country::FR => "FRA",
            Country::GF => "GUF",
            Country::PF => "PYF",
            Country::TF => "ATF",
            Country::GA => "GAB",
            Country::GM => "GMB",
            Country::GE => "GEO",
            Country::DE => "DEU",
            Country::GH => "GHA",
            Country::GI => "GIB",
            Country::GR => "GRC",
            Country::GL => "GRL",
            Country::GD => "GRD",
            Country::GP => "GLP",
            Country::GU => "GUM",
            Country::GT => "GTM",
            Country::GG => "GGY",
            Country::GN => "GIN",
            Country::GW => "GNB",
            Country::GY => "GUY",
            Country::HT => "HTI",
            Country::HM => "HMD",
            Country::HN => "HND",
            Country::HK => "HKG",
            Country::HU => "HUN",
            Country::IS => "ISL",
            Country::IN => "IND",
            Country::ID => "IDN",
            Country::IR => "IRN",
            Country::IQ => "IRQ",
            Country::IE => "IRL",
            Country::IM => "IMN",
            Country::IL => "ISR",
            Country::IT => "ITA",
            Country::JM => "JAM",
            Country::JP => "JPN",
            Country::JE => "JEY",
            Country::JO => "JOR",
            Country::KZ => "KAZ",
            Country::KE => "KEN",
            Country::KI => "KIR",
            Country::KW => "KWT",
            Country::KG => "KGZ",
            Country::LA => "LAO",
            Country::LV => "LVA",
            Country::LB => "LBN",
            Country::LS => "LSO",
            Country::LR => "LBR",
            Country::LY => "LBY",
            Country::LI => "LIE",
            Country::LT => "LTU",
            Country::LU => "LUX",
            Country::MO => "MAC",
            Country::MG => "MDG",
            Country::MW => "MWI",
            Country::MY => "MYS",
            Country::MV => "MDV",
            Country::ML => "MLI",
            Country::MT => "MLT",
            Country::MH => "MHL",
            Country::MQ => "MTQ",
            Country::MR => "MRT",
            Country::MU => "MUS",
            Country::YT => "MYT",
            Country::MX => "MEX",
            Country::FM => "FSM",
            Country::MD => "MDA",
            Country::MC => "MCO",
            Country::MN => "MNG",
            Country::ME => "MNE",
            Country::MS => "MSR",
            Country::MA => "MAR",
            Country::MZ => "MOZ",
            Country::MM => "MMR",
            Country::NA => "NAM",
            Country::NR => "NRU",
            Country::NP => "NPL",
            Country::NL => "NLD",
            Country::NC => "NCL",
            Country::NZ => "NZL",
            Country::NI => "NIC",
            Country::NE => "NER",
            Country::NG => "NGA",
            Country::NU => "NIU",
            Country::NF => "NFK",
            Country::KP => "PRK",
            Country::MK => "MKD",
            Country::MP => "MNP",
            Country::NO => "NOR",
            Country::OM => "OMN",
            Country::PK => "PAK",
            Country::PW => "PLW",
            Country::PS => "PSE",
            Country::PA => "PAN",
            Country::PG => "PNG",
            Country::PY => "PRY",
            Country::PE => "PER",
            Country::PH => "PHL",
            Country::PN => "PCN",
            Country::PL => "POL",
            Country::PT => "PRT",
            Country::PR => "PRI",
            Country::QA => "QAT",
            Country::RE => "REU",
            Country::RO => "ROU",
            Country::RU => "RUS",
            Country::RW => "RWA",
            Country::WS => "WSM",
            Country::SM => "SMR",
            Country::ST => "STP",
            Country::SA => "SAU",
            Country::SN => "SEN",
            Country::RS => "SRB",
            Country::SC => "SYC",
            Country::SL => "SLE",
            Country::SG => "SGP",
            Country::SX => "SXM",
            Country::SK => "SVK",
            Country::SI => "SVN",
            Country::SB => "SLB",
            Country::SO => "SOM",
            Country::ZA => "ZAF",
            Country::GS => "SGS",
            Country::KR => "KOR",
            Country::SS => "SSD",
            Country::ES => "ESP",
            Country::LK => "LKA",
            Country::BL => "BLM",
            Country::SH => "SHN",
            Country::KN => "KNA",
            Country::LC => "LCA",
            Country::MF => "MAF",
            Country::PM => "SPM",
            Country::VC => "VCT",
            Country::SD => "SDN",
            Country::SR => "SUR",
            Country::SJ => "SJM",
            Country::SE => "SWE",
            Country::CH => "CHE",
            Country::SY => "SYR",
            Country::TW => "TWN",
            Country::TJ => "TJK",
            Country::TZ => "TZA",
            Country::TH => "THA",
            Country::TL => "TLS",
            Country::TG => "TGO",
            Country::TK => "TKL",
            Country::TO => "TON",
            Country::TT => "TTO",
            Country::TN => "TUN",
            Country::TR => "TUR",
            Country::TM => "TKM",
            Country::TC => "TCA",
            Country::TV => "TUV",
            Country::UM => "UMI",
            Country::VI => "VIR",
            Country::UG => "UGA",
            Country::UA => "UKR",
            Country::AE => "ARE",
            Country::GB => "GBR",
            Country::US => "USA",
            Country::UY => "URY",
            Country::UZ => "UZB",
            Country::VU => "VUT",
            Country::VA => "VAT",
            Country::VE => "VEN",
            Country::VN => "VNM",
            Country::WF => "WLF",
            Country::EH => "ESH",
            Country::YE => "YEM",
            Country::ZM => "ZMB",
            Country::ZW => "ZWE",

            // Abnormal
            Country::EU => "EUR",
            Country::XX => "XXX",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CountryList(pub Vec<Country>);

impl From<CountryList> for Vec<String> {
    fn from(countries: CountryList) -> Self {
        countries.0.iter().map(|c| c.to_string()).collect()
    }
}

impl CountryList {
    pub fn first(&self) -> Option<&Country> {
        self.0.first()
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Text, Pg> for Country {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        write!(out, "{self}")?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Text, Pg> for Country {
    fn from_sql(bytes: diesel::pg::PgValue) -> deserialize::Result<Self> {
        let s = std::str::from_utf8(bytes.as_bytes())?;
        s.parse().map_err(|_| "Unrecognized country".into())
    }
}
