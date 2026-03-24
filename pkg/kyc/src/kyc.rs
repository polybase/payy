// lint-long-file-override allow-max-lines=400
use super::{
    Error, IDDocument, IDDocumentField, IDKind, Result, deserialize_document_field_opt, ok_field,
    serialize_document_field_opt,
};
use currency::{Country, CountryList};
#[cfg(feature = "diesel")]
use diesel::{
    AsExpression,
    deserialize::{self, FromSql, FromSqlRow},
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Jsonb,
};
use phonenumber::parse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(feature = "diesel")]
use std::io::Write;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature="diesel",diesel(sql_type = diesel::sql_types::Jsonb))]
pub struct Kyc {
    pub firstname: Option<String>,
    pub middlename: Option<String>,
    pub lastname: Option<String>,
    pub dob: Option<chrono::NaiveDate>,
    pub occupation: Option<String>,
    pub addressstreet: Option<String>,
    pub addresscity: Option<String>,
    pub addressstate: Option<String>,
    pub addresscountry: Option<Country>,
    pub addresspostalcode: Option<String>,
    pub nationalities: Option<CountryList>,
    #[serde(default)]
    pub documents: Option<HashMap<Country, HashMap<IDKind, IDDocument>>>,
    pub phone: Option<String>,
    pub phoneverified: Option<bool>,
    pub email: Option<String>,
    pub emailverified: Option<bool>,
    pub nationalid: Option<String>,
    pub pep: Option<bool>,
    pub civilstate: Option<String>,
    pub fatca: Option<bool>,
    pub uif: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        default,
        serialize_with = "serialize_document_field_opt",
        deserialize_with = "deserialize_document_field_opt"
    )]
    pub selfiefront: Option<IDDocumentField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        default,
        serialize_with = "serialize_document_field_opt",
        deserialize_with = "deserialize_document_field_opt"
    )]
    pub selfieleft: Option<IDDocumentField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(
        default,
        serialize_with = "serialize_document_field_opt",
        deserialize_with = "deserialize_document_field_opt"
    )]
    pub selfieright: Option<IDDocumentField>,
}

impl Kyc {
    pub fn merge(&self, kyc: &Kyc) -> Self {
        let mut merged_documents = self.documents.clone().unwrap_or_default();

        if let Some(new_documents) = &kyc.documents {
            for (country, ids) in new_documents {
                let entry = merged_documents.entry(*country).or_default();
                for (id_kind, doc) in ids {
                    let sanitized_doc = match (doc.front.as_ref(), doc.back.as_ref()) {
                        (Some(IDDocumentField::Bytes(_)), _)
                        | (_, Some(IDDocumentField::Bytes(_))) => IDDocument {
                            front: None,
                            back: None,
                        },
                        _ => doc.clone(),
                    };
                    entry.insert(*id_kind, sanitized_doc);
                }
            }
        }

        Kyc {
            firstname: kyc.firstname.clone().or_else(|| self.firstname.clone()),
            middlename: kyc.middlename.clone().or_else(|| self.middlename.clone()),
            lastname: kyc.lastname.clone().or_else(|| self.lastname.clone()),
            dob: kyc.dob.or(self.dob),
            occupation: kyc.occupation.clone().or_else(|| self.occupation.clone()),
            addressstreet: kyc
                .addressstreet
                .clone()
                .or_else(|| self.addressstreet.clone()),
            addresscity: kyc.addresscity.clone().or_else(|| self.addresscity.clone()),
            addressstate: kyc
                .addressstate
                .clone()
                .or_else(|| self.addressstate.clone()),
            addresscountry: kyc.addresscountry.or(self.addresscountry),
            addresspostalcode: kyc
                .addresspostalcode
                .clone()
                .or_else(|| self.addresspostalcode.clone()),
            nationalities: kyc
                .nationalities
                .clone()
                .or_else(|| self.nationalities.clone()),
            phone: kyc.phone.clone().or_else(|| self.phone.clone()),
            phoneverified: kyc.phoneverified.or(self.phoneverified),
            email: kyc.email.clone().or_else(|| self.email.clone()),
            emailverified: kyc.emailverified.or(self.emailverified),
            nationalid: kyc.nationalid.clone().or_else(|| self.nationalid.clone()),
            pep: kyc.pep.or(self.pep),
            civilstate: kyc.civilstate.clone().or_else(|| self.civilstate.clone()),
            fatca: kyc.fatca.or(self.fatca),
            uif: kyc.uif.or(self.uif),
            documents: Some(merged_documents),
            selfiefront: kyc.selfiefront.clone().or_else(|| self.selfiefront.clone()),
            selfieleft: kyc.selfieleft.clone().or_else(|| self.selfieleft.clone()),
            selfieright: kyc.selfieright.clone().or_else(|| self.selfieright.clone()),
        }
    }

    pub fn sanitize_for_storage(&self) -> Self {
        let mut sanitized = self.clone();

        if let Some(mut docs) = sanitized.documents.take() {
            for (_country, id_map) in docs.iter_mut() {
                for (_id_kind, doc) in id_map.iter_mut() {
                    if let Some(IDDocumentField::Bytes(_)) = doc.front {
                        doc.front = None;
                    }
                    if let Some(IDDocumentField::Bytes(_)) = doc.back {
                        doc.back = None;
                    }
                }
            }
            sanitized.documents = Some(docs);
        }

        sanitized
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Jsonb, Pg> for Kyc {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, self)?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Jsonb, Pg> for Kyc {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let bytes = bytes.as_bytes();
        if bytes.is_empty() {
            return Ok(Kyc::default());
        }
        let json_bytes = &bytes[1..];
        serde_json::from_slice(json_bytes).map_err(Into::into)
    }
}

impl Kyc {
    pub fn get_firstname(&self) -> Result<String> {
        ok_field(&self.firstname, "firstname")
    }

    pub fn get_lastname(&self) -> Result<String> {
        ok_field(&self.lastname, "lastname")
    }

    pub fn get_dob(&self) -> Result<chrono::NaiveDate> {
        ok_field(&self.dob, "dob")
    }

    pub fn get_occupation(&self) -> Result<String> {
        ok_field(&self.occupation, "occupation")
    }

    pub fn get_addressstreet(&self) -> Result<String> {
        ok_field(&self.addressstreet, "addressstreet")
    }

    pub fn get_addresscity(&self) -> Result<String> {
        ok_field(&self.addresscity, "addresscity")
    }

    pub fn get_addressstate(&self) -> Result<String> {
        ok_field(&self.addressstate, "addressstate")
    }

    pub fn get_addresscountry(&self) -> Result<Country> {
        ok_field(&self.addresscountry, "addresscountry")
    }

    pub fn get_addresspostalcode(&self) -> Result<String> {
        ok_field(&self.addresspostalcode, "addresspostalcode")
    }

    pub fn get_nationalities(&self) -> Result<CountryList> {
        let nationalities = ok_field(&self.nationalities, "nationalities")?;
        if nationalities.0.is_empty() {
            return Err(Error::MissingKYCField("nationalities".to_string()));
        }
        Ok(nationalities)
    }

    pub fn get_nationality_one(&self) -> Result<Country> {
        ok_field(&self.nationalities, "nationalities")?
            .0
            .first()
            .cloned()
            .ok_or(Error::MissingKYCField("nationalities".to_string()))
    }

    pub fn get_document_by_country(&self, country: Country) -> Result<(IDKind, IDDocument)> {
        let documents = ok_field(&self.documents, "documents")?;
        documents
            .get(&country)
            .and_then(|docs| docs.iter().next())
            .map(|(kind, doc)| (*kind, doc.clone()))
            .ok_or(Error::MissingKYCField("documents".to_string()))
    }

    pub fn get_all_documents(&self) -> Result<Vec<IDDocument>> {
        let documents = ok_field(&self.documents, "documents")?;
        let mut all = Vec::new();

        for doc_map in documents.values() {
            for doc in doc_map.values() {
                all.push(doc.clone());
            }
        }

        if all.is_empty() {
            return Err(Error::MissingKYCField("documents".to_string()));
        }

        Ok(all)
    }

    pub fn get_documents_by_country_and_kind(
        &self,
        country: Country,
        kind: IDKind,
    ) -> Result<IDDocument> {
        let documents = ok_field(&self.documents, "documents")?;
        documents
            .get(&country)
            .and_then(|docs| docs.get(&kind))
            .cloned()
            .ok_or(Error::MissingKYCField("documents".to_string()))
    }

    pub fn get_one_of_document_kinds(
        &self,
        country: Country,
        kinds: &[IDKind],
    ) -> Result<IDDocument> {
        let documents = ok_field(&self.documents, "documents")?;
        for kind in kinds {
            if let Some(doc) = documents.get(&country).and_then(|docs| docs.get(kind)) {
                return Ok(doc.clone());
            }
        }
        Err(Error::MissingKYCField("documents".to_string()))
    }

    pub fn get_documents_countries(&self) -> Result<Vec<Country>> {
        let documents = ok_field(&self.documents, "documents")?;
        Ok(documents.keys().cloned().collect())
    }

    pub fn get_documents_countries_one(&self) -> Result<Country> {
        let countries = self.get_documents_countries()?;
        countries
            .first()
            .cloned()
            .ok_or(Error::MissingKYCField("documents".to_string()))
    }

    pub fn get_phone(&self) -> Result<String> {
        ok_field(&self.phone, "phone")
    }

    pub fn get_phone_parts(&self) -> Result<(String, String)> {
        let phone = self.get_phone()?;

        // Remove leading zeros and ensure phone has a + prefix
        let phone_with_prefix = if !phone.starts_with('+') {
            format!("+{}", phone.trim_start_matches('0'))
        } else {
            format!("+{}", phone.trim_start_matches(['+', '0']))
        };

        // Try to parse without country first, then with country if that fails
        let parsed = parse(None, phone_with_prefix).map_err(|_| Error::InvalidPhoneFormat)?;

        // Get the country code (as u32) and convert to string
        let country_code = parsed.country().code().to_string();

        // Get the national number (without country code)
        let national_number = parsed.national().value().to_string();

        Ok((country_code, national_number))
    }

    pub fn get_phone_verified(&self) -> Result<bool> {
        ok_field(&self.phoneverified, "phoneverified")
    }

    pub fn get_email(&self) -> Result<String> {
        ok_field(&self.email, "email")
    }

    pub fn get_email_verified(&self) -> Result<bool> {
        ok_field(&self.emailverified, "emailverified")
    }

    pub fn get_nationalid(&self) -> Result<String> {
        ok_field(&self.nationalid, "nationalid")
    }

    pub fn get_civil_state(&self) -> Result<String> {
        ok_field(&self.civilstate, "civilstate")
    }

    pub fn get_fatca(&self) -> Result<bool> {
        ok_field(&self.fatca, "fatca")
    }

    pub fn get_uif(&self) -> Result<bool> {
        ok_field(&self.uif, "uif")
    }

    pub fn get_pep(&self) -> Result<bool> {
        ok_field(&self.pep, "pep")
    }
}
