use crate::contacts::{
    Contact, ContactEmail, ContactPhone, ContactSource, ContactsError, ContactsResult,
};
use crate::oauth2::TokenManager;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use percent_encoding;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

/// Contact provider trait
#[async_trait]
pub trait ContactsProvider: Send + Sync {
    /// Fetch all contacts from the provider
    async fn fetch_contacts(&self, account_id: &str) -> ContactsResult<Vec<Contact>>;

    /// Fetch contacts with pagination
    async fn fetch_contacts_page(
        &self,
        account_id: &str,
        page_token: Option<String>,
    ) -> ContactsResult<ContactsPage>;

    /// Create a new contact
    async fn create_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact>;

    /// Update an existing contact
    async fn update_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact>;

    /// Delete a contact
    async fn delete_contact(&self, account_id: &str, contact_id: &str) -> ContactsResult<()>;

    /// Get provider name
    fn provider_name(&self) -> &str;
}

/// Paginated contacts response
#[derive(Debug, Clone)]
pub struct ContactsPage {
    pub contacts: Vec<Contact>,
    pub next_page_token: Option<String>,
    pub total_items: Option<usize>,
}

/// Google Contacts provider
pub struct GoogleContactsProvider {
    http_client: HttpClient,
    token_manager: TokenManager,
}

impl GoogleContactsProvider {
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            http_client: HttpClient::new(),
            token_manager,
        }
    }

    async fn get_access_token(&self, account_id: &str) -> ContactsResult<String> {
        // TEMPORARY: Load token directly from file to bypass TokenManager cache issue
        let config_dir = dirs::config_dir()
            .ok_or_else(|| ContactsError::AuthError("Cannot find config directory".to_string()))?
            .join("comunicado");
        let token_file = config_dir.join(format!("{}.access.token", account_id));
        
        if token_file.exists() {
            let encoded_token = std::fs::read_to_string(&token_file)
                .map_err(|e| ContactsError::AuthError(format!("Failed to read token file: {}", e)))?;
            let encoded_token = encoded_token.trim();
            
            use base64::{Engine as _, engine::general_purpose};
            let decoded_token = general_purpose::STANDARD.decode(encoded_token)
                .map_err(|e| ContactsError::AuthError(format!("Failed to decode token: {}", e)))?;
            let token_str = String::from_utf8(decoded_token)
                .map_err(|e| ContactsError::AuthError(format!("Invalid token encoding: {}", e)))?;
                
            println!("🔍 DEBUG: Using file token for contacts (first 50 chars): {}", &token_str[..50.min(token_str.len())]);
            return Ok(token_str);
        }
        
        // Fallback to TokenManager (original code)
        let token = self
            .token_manager
            .get_valid_access_token(account_id)
            .await
            .map_err(|e| ContactsError::AuthError(e.to_string()))?
            .ok_or_else(|| ContactsError::AuthError("No access token found".to_string()))?;

        println!("🔍 DEBUG: Using TokenManager token for contacts (first 50 chars): {}", &token.token[..50.min(token.token.len())]);
        Ok(token.token)
    }
}

#[async_trait]
impl ContactsProvider for GoogleContactsProvider {
    async fn fetch_contacts(&self, account_id: &str) -> ContactsResult<Vec<Contact>> {
        let mut all_contacts = Vec::new();
        let mut page_token = None;

        loop {
            let page = self.fetch_contacts_page(account_id, page_token).await?;
            all_contacts.extend(page.contacts);

            if page.next_page_token.is_none() {
                break;
            }
            page_token = page.next_page_token;
        }

        Ok(all_contacts)
    }

    async fn fetch_contacts_page(
        &self,
        account_id: &str,
        page_token: Option<String>,
    ) -> ContactsResult<ContactsPage> {
        let access_token = self.get_access_token(account_id).await?;

        let mut url = "https://people.googleapis.com/v1/people/me/connections".to_string();
        let mut params = vec![
            (
                "personFields",
                "names,emailAddresses,phoneNumbers,organizations,photos,metadata",
            ),
            ("pageSize", "1000"),
        ];

        if let Some(token) = &page_token {
            params.push(("pageToken", token));
        }

        let query_string = params
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    k,
                    percent_encoding::utf8_percent_encode(v, percent_encoding::NON_ALPHANUMERIC)
                )
            })
            .collect::<Vec<_>>()
            .join("&");

        url.push_str(&format!("?{}", query_string));

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Google Contacts API error: {}",
                error_text
            )));
        }

        let data: GoogleContactsResponse = response.json().await?;

        let mut contacts = Vec::new();
        if let Some(connections) = data.connections {
            for person in connections {
                if let Ok(contact) = self.convert_google_person_to_contact(person, account_id) {
                    contacts.push(contact);
                }
            }
        }

        Ok(ContactsPage {
            contacts,
            next_page_token: data.next_page_token,
            total_items: data.total_items,
        })
    }

    async fn create_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact> {
        let access_token = self.get_access_token(account_id).await?;

        let google_person = self.convert_contact_to_google_person(contact);

        let response = self
            .http_client
            .post("https://people.googleapis.com/v1/people:createContact")
            .bearer_auth(&access_token)
            .json(&google_person)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to create contact: {}",
                error_text
            )));
        }

        let created_person: GooglePerson = response.json().await?;
        self.convert_google_person_to_contact(created_person, account_id)
    }

    async fn update_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact> {
        let access_token = self.get_access_token(account_id).await?;

        let google_person = self.convert_contact_to_google_person(contact);
        let url = format!("https://people.googleapis.com/v1/{}", contact.external_id);

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&access_token)
            .header("Content-Type", "application/json")
            .query(&[(
                "updatePersonFields",
                "names,emailAddresses,phoneNumbers,organizations",
            )])
            .json(&google_person)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to update contact: {}",
                error_text
            )));
        }

        let updated_person: GooglePerson = response.json().await?;
        self.convert_google_person_to_contact(updated_person, account_id)
    }

    async fn delete_contact(&self, account_id: &str, contact_id: &str) -> ContactsResult<()> {
        let access_token = self.get_access_token(account_id).await?;

        let url = format!(
            "https://people.googleapis.com/v1/{}:deleteContact",
            contact_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to delete contact: {}",
                error_text
            )));
        }

        Ok(())
    }

    fn provider_name(&self) -> &str {
        "Google Contacts"
    }
}

impl GoogleContactsProvider {
    fn convert_google_person_to_contact(
        &self,
        person: GooglePerson,
        account_id: &str,
    ) -> ContactsResult<Contact> {
        let resource_name = person
            .resource_name
            .ok_or_else(|| ContactsError::InvalidData("Missing resource name".to_string()))?;

        let source = ContactSource::Google {
            account_id: account_id.to_string(),
        };

        // Get display name
        let display_name = if let Some(names) = &person.names {
            names
                .first()
                .and_then(|n| n.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string())
        } else {
            "Unknown".to_string()
        };

        let mut contact = Contact::new(resource_name, source, display_name);

        // Set names
        if let Some(names) = person.names {
            if let Some(name) = names.first() {
                contact.first_name = name.given_name.clone();
                contact.last_name = name.family_name.clone();
            }
        }

        // Set emails
        if let Some(emails) = person.email_addresses {
            for email in emails {
                if let Some(address) = email.value {
                    let label = email.r#type.unwrap_or_else(|| "other".to_string());
                    let is_primary = email
                        .metadata
                        .as_ref()
                        .and_then(|m| m.primary)
                        .unwrap_or(false);

                    contact.emails.push(ContactEmail {
                        address,
                        label,
                        is_primary,
                    });
                }
            }
        }

        // Set phones
        if let Some(phones) = person.phone_numbers {
            for phone in phones {
                if let Some(number) = phone.value {
                    let label = phone.r#type.unwrap_or_else(|| "other".to_string());
                    let is_primary = phone
                        .metadata
                        .as_ref()
                        .and_then(|m| m.primary)
                        .unwrap_or(false);

                    contact.phones.push(ContactPhone {
                        number,
                        label,
                        is_primary,
                    });
                }
            }
        }

        // Set organization
        if let Some(organizations) = person.organizations {
            if let Some(org) = organizations.first() {
                contact.company = org.name.clone();
                contact.job_title = org.title.clone();
            }
        }

        // Set photo
        if let Some(photos) = person.photos {
            if let Some(photo) = photos.first() {
                contact.photo_url = photo.url.clone();
            }
        }

        // Set metadata
        if let Some(metadata) = person.metadata {
            contact.etag = metadata.etag;
            if let Some(sources) = metadata.sources {
                if let Some(source) = sources.first() {
                    if let Some(update_time) = &source.update_time {
                        if let Ok(dt) = DateTime::parse_from_rfc3339(update_time) {
                            contact.updated_at = dt.with_timezone(&Utc);
                        }
                    }
                }
            }
        }

        Ok(contact)
    }

    fn convert_contact_to_google_person(&self, contact: &Contact) -> GooglePerson {
        let mut person = GooglePerson {
            resource_name: Some(contact.external_id.clone()),
            etag: contact.etag.clone(),
            metadata: None,
            names: None,
            email_addresses: None,
            phone_numbers: None,
            organizations: None,
            photos: None,
        };

        // Convert names
        if contact.first_name.is_some() || contact.last_name.is_some() {
            person.names = Some(vec![GoogleName {
                display_name: Some(contact.display_name.clone()),
                given_name: contact.first_name.clone(),
                family_name: contact.last_name.clone(),
                metadata: Some(GoogleFieldMetadata {
                    primary: Some(true),
                    source: None,
                }),
            }]);
        }

        // Convert emails
        if !contact.emails.is_empty() {
            person.email_addresses = Some(
                contact
                    .emails
                    .iter()
                    .map(|email| GoogleEmail {
                        value: Some(email.address.clone()),
                        r#type: Some(email.label.clone()),
                        display_name: None,
                        metadata: Some(GoogleFieldMetadata {
                            primary: Some(email.is_primary),
                            source: None,
                        }),
                    })
                    .collect(),
            );
        }

        // Convert phones
        if !contact.phones.is_empty() {
            person.phone_numbers = Some(
                contact
                    .phones
                    .iter()
                    .map(|phone| GooglePhone {
                        value: Some(phone.number.clone()),
                        r#type: Some(phone.label.clone()),
                        canonical_form: None,
                        metadata: Some(GoogleFieldMetadata {
                            primary: Some(phone.is_primary),
                            source: None,
                        }),
                    })
                    .collect(),
            );
        }

        // Convert organization
        if contact.company.is_some() || contact.job_title.is_some() {
            person.organizations = Some(vec![GoogleOrganization {
                name: contact.company.clone(),
                title: contact.job_title.clone(),
                r#type: Some("work".to_string()),
                metadata: Some(GoogleFieldMetadata {
                    primary: Some(true),
                    source: None,
                }),
            }]);
        }

        person
    }
}

/// Microsoft Graph Contacts provider
pub struct OutlookContactsProvider {
    http_client: HttpClient,
    token_manager: TokenManager,
}

impl OutlookContactsProvider {
    pub fn new(token_manager: TokenManager) -> Self {
        Self {
            http_client: HttpClient::new(),
            token_manager,
        }
    }

    async fn get_access_token(&self, account_id: &str) -> ContactsResult<String> {
        let token = self
            .token_manager
            .get_valid_access_token(account_id)
            .await
            .map_err(|e| ContactsError::AuthError(e.to_string()))?
            .ok_or_else(|| ContactsError::AuthError("No access token found".to_string()))?;


        Ok(token.token)
    }
}

#[async_trait]
impl ContactsProvider for OutlookContactsProvider {
    async fn fetch_contacts(&self, account_id: &str) -> ContactsResult<Vec<Contact>> {
        let mut all_contacts = Vec::new();
        let mut skip = 0;
        let top = 1000;

        loop {
            let page = self
                .fetch_contacts_page(account_id, Some(skip.to_string()))
                .await?;
            let page_size = page.contacts.len();
            all_contacts.extend(page.contacts);

            if page_size < top {
                break;
            }
            skip += top;
        }

        Ok(all_contacts)
    }

    async fn fetch_contacts_page(
        &self,
        account_id: &str,
        skip_token: Option<String>,
    ) -> ContactsResult<ContactsPage> {
        let access_token = self.get_access_token(account_id).await?;

        let mut url = "https://graph.microsoft.com/v1.0/me/contacts".to_string();
        let skip = skip_token.unwrap_or_else(|| "0".to_string());

        url.push_str(&format!("?$top=1000&$skip={}", skip));

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Microsoft Graph API error: {}",
                error_text
            )));
        }

        let data: OutlookContactsResponse = response.json().await?;

        let mut contacts = Vec::new();
        for outlook_contact in data.value {
            if let Ok(contact) =
                self.convert_outlook_contact_to_contact(outlook_contact, account_id)
            {
                contacts.push(contact);
            }
        }

        Ok(ContactsPage {
            contacts,
            next_page_token: data.odata_next_link,
            total_items: None,
        })
    }

    async fn create_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact> {
        let access_token = self.get_access_token(account_id).await?;

        let outlook_contact = self.convert_contact_to_outlook_contact(contact);

        let response = self
            .http_client
            .post("https://graph.microsoft.com/v1.0/me/contacts")
            .bearer_auth(&access_token)
            .json(&outlook_contact)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to create contact: {}",
                error_text
            )));
        }

        let created_contact: OutlookContact = response.json().await?;
        self.convert_outlook_contact_to_contact(created_contact, account_id)
    }

    async fn update_contact(&self, account_id: &str, contact: &Contact) -> ContactsResult<Contact> {
        let access_token = self.get_access_token(account_id).await?;

        let outlook_contact = self.convert_contact_to_outlook_contact(contact);
        let url = format!(
            "https://graph.microsoft.com/v1.0/me/contacts/{}",
            contact.external_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&access_token)
            .json(&outlook_contact)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to update contact: {}",
                error_text
            )));
        }

        let updated_contact: OutlookContact = response.json().await?;
        self.convert_outlook_contact_to_contact(updated_contact, account_id)
    }

    async fn delete_contact(&self, account_id: &str, contact_id: &str) -> ContactsResult<()> {
        let access_token = self.get_access_token(account_id).await?;

        let url = format!(
            "https://graph.microsoft.com/v1.0/me/contacts/{}",
            contact_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ContactsError::ApiError(format!(
                "Failed to delete contact: {}",
                error_text
            )));
        }

        Ok(())
    }

    fn provider_name(&self) -> &str {
        "Outlook Contacts"
    }
}

impl OutlookContactsProvider {
    fn convert_outlook_contact_to_contact(
        &self,
        outlook_contact: OutlookContact,
        account_id: &str,
    ) -> ContactsResult<Contact> {
        let id = outlook_contact
            .id
            .ok_or_else(|| ContactsError::InvalidData("Missing contact ID".to_string()))?;

        let source = ContactSource::Outlook {
            account_id: account_id.to_string(),
        };

        let display_name = outlook_contact
            .display_name
            .unwrap_or_else(|| "Unknown".to_string());

        let mut contact = Contact::new(id, source, display_name);

        contact.first_name = outlook_contact.given_name;
        contact.last_name = outlook_contact.surname;
        contact.company = outlook_contact.company_name;
        contact.job_title = outlook_contact.job_title;

        // Convert email addresses
        if let Some(email_addresses) = outlook_contact.email_addresses {
            for email in email_addresses {
                if let Some(address) = email.address {
                    contact.emails.push(ContactEmail {
                        address,
                        label: email.name.unwrap_or_else(|| "other".to_string()),
                        is_primary: false, // Outlook doesn't provide primary info in this format
                    });
                }
            }
        }

        // Convert phone numbers
        if let Some(business_phones) = outlook_contact.business_phones {
            for phone in business_phones {
                contact
                    .phones
                    .push(ContactPhone::new(phone, "work".to_string()));
            }
        }

        if let Some(home_phones) = outlook_contact.home_phones {
            for phone in home_phones {
                contact
                    .phones
                    .push(ContactPhone::new(phone, "home".to_string()));
            }
        }

        if let Some(mobile_phone) = outlook_contact.mobile_phone {
            contact
                .phones
                .push(ContactPhone::new(mobile_phone, "mobile".to_string()));
        }

        // Set timestamps
        if let Some(created_time) = outlook_contact.created_date_time {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&created_time) {
                contact.created_at = dt.with_timezone(&Utc);
            }
        }

        if let Some(modified_time) = outlook_contact.last_modified_date_time {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&modified_time) {
                contact.updated_at = dt.with_timezone(&Utc);
            }
        }

        Ok(contact)
    }

    fn convert_contact_to_outlook_contact(&self, contact: &Contact) -> OutlookContact {
        let mut outlook_contact = OutlookContact {
            id: Some(contact.external_id.clone()),
            display_name: Some(contact.display_name.clone()),
            given_name: contact.first_name.clone(),
            surname: contact.last_name.clone(),
            company_name: contact.company.clone(),
            job_title: contact.job_title.clone(),
            email_addresses: None,
            business_phones: None,
            home_phones: None,
            mobile_phone: None,
            created_date_time: None,
            last_modified_date_time: None,
        };

        // Convert emails
        if !contact.emails.is_empty() {
            outlook_contact.email_addresses = Some(
                contact
                    .emails
                    .iter()
                    .map(|email| OutlookEmailAddress {
                        address: Some(email.address.clone()),
                        name: Some(email.label.clone()),
                    })
                    .collect(),
            );
        }

        // Convert phones
        let mut business_phones = Vec::new();
        let mut home_phones = Vec::new();
        let mut mobile_phone = None;

        for phone in &contact.phones {
            match phone.label.as_str() {
                "work" | "business" => business_phones.push(phone.number.clone()),
                "home" => home_phones.push(phone.number.clone()),
                "mobile" | "cell" => {
                    if mobile_phone.is_none() {
                        mobile_phone = Some(phone.number.clone());
                    }
                }
                _ => business_phones.push(phone.number.clone()),
            }
        }

        if !business_phones.is_empty() {
            outlook_contact.business_phones = Some(business_phones);
        }

        if !home_phones.is_empty() {
            outlook_contact.home_phones = Some(home_phones);
        }

        outlook_contact.mobile_phone = mobile_phone;

        outlook_contact
    }
}

// Google Contacts API types
#[derive(Debug, Deserialize)]
struct GoogleContactsResponse {
    connections: Option<Vec<GooglePerson>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "totalItems")]
    total_items: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePerson {
    #[serde(rename = "resourceName")]
    resource_name: Option<String>,
    etag: Option<String>,
    metadata: Option<GooglePersonMetadata>,
    names: Option<Vec<GoogleName>>,
    #[serde(rename = "emailAddresses")]
    email_addresses: Option<Vec<GoogleEmail>>,
    #[serde(rename = "phoneNumbers")]
    phone_numbers: Option<Vec<GooglePhone>>,
    organizations: Option<Vec<GoogleOrganization>>,
    photos: Option<Vec<GooglePhoto>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePersonMetadata {
    sources: Option<Vec<GoogleSource>>,
    etag: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleSource {
    #[serde(rename = "updateTime")]
    update_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleName {
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "givenName")]
    given_name: Option<String>,
    #[serde(rename = "familyName")]
    family_name: Option<String>,
    metadata: Option<GoogleFieldMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleEmail {
    value: Option<String>,
    r#type: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    metadata: Option<GoogleFieldMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePhone {
    value: Option<String>,
    r#type: Option<String>,
    #[serde(rename = "canonicalForm")]
    canonical_form: Option<String>,
    metadata: Option<GoogleFieldMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleOrganization {
    name: Option<String>,
    title: Option<String>,
    r#type: Option<String>,
    metadata: Option<GoogleFieldMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GooglePhoto {
    url: Option<String>,
    metadata: Option<GoogleFieldMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleFieldMetadata {
    primary: Option<bool>,
    source: Option<GoogleSource>,
}

// Microsoft Graph Contacts API types
#[derive(Debug, Deserialize)]
struct OutlookContactsResponse {
    #[serde(rename = "@odata.nextLink")]
    odata_next_link: Option<String>,
    value: Vec<OutlookContact>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutlookContact {
    id: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "givenName")]
    given_name: Option<String>,
    surname: Option<String>,
    #[serde(rename = "companyName")]
    company_name: Option<String>,
    #[serde(rename = "jobTitle")]
    job_title: Option<String>,
    #[serde(rename = "emailAddresses")]
    email_addresses: Option<Vec<OutlookEmailAddress>>,
    #[serde(rename = "businessPhones")]
    business_phones: Option<Vec<String>>,
    #[serde(rename = "homePhones")]
    home_phones: Option<Vec<String>>,
    #[serde(rename = "mobilePhone")]
    mobile_phone: Option<String>,
    #[serde(rename = "createdDateTime")]
    created_date_time: Option<String>,
    #[serde(rename = "lastModifiedDateTime")]
    last_modified_date_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutlookEmailAddress {
    address: Option<String>,
    name: Option<String>,
}
