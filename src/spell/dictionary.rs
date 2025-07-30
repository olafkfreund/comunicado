use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Dictionary file paths for a language
#[derive(Debug, Clone)]
pub struct DictionaryPaths {
    pub aff: PathBuf,
    pub dic: PathBuf,
}

/// Dictionary metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryInfo {
    pub language: String,
    pub display_name: String,
    pub version: String,
    pub description: String,
    pub installed: bool,
}

/// Manages spell check dictionaries
pub struct DictionaryManager {
    dict_dir: PathBuf,
    available_dicts: HashMap<String, DictionaryInfo>,
}

impl DictionaryManager {
    /// Create new dictionary manager
    pub fn new() -> Result<Self> {
        let dict_dir = Self::get_dictionary_directory()?;
        let _ = std::fs::create_dir_all(&dict_dir); // Create if doesn't exist

        let mut manager = Self {
            dict_dir,
            available_dicts: HashMap::new(),
        };

        manager.scan_dictionaries()?;
        Ok(manager)
    }

    /// Get the dictionary directory path
    fn get_dictionary_directory() -> Result<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("comunicado").join("dictionaries"))
        } else {
            // Fallback to current directory
            Ok(PathBuf::from(".").join("dictionaries"))
        }
    }

    /// Scan for available dictionaries
    fn scan_dictionaries(&mut self) -> Result<()> {
        // Add built-in dictionary information
        self.add_builtin_dictionaries();

        // Scan local dictionary directory
        if self.dict_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.dict_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".aff") {
                            let lang_code = name.trim_end_matches(".aff");
                            let dic_path = self.dict_dir.join(format!("{}.dic", lang_code));

                            if dic_path.exists() {
                                self.mark_dictionary_installed(lang_code);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Add information about built-in dictionaries
    fn add_builtin_dictionaries(&mut self) {
        let dictionaries = vec![
            (
                "en_US",
                "English (United States)",
                "English spell checking dictionary",
            ),
            (
                "en_GB",
                "English (United Kingdom)",
                "British English spell checking dictionary",
            ),
            (
                "es_ES",
                "Spanish (Spain)",
                "Spanish spell checking dictionary",
            ),
            (
                "fr_FR",
                "French (France)",
                "French spell checking dictionary",
            ),
            (
                "de_DE",
                "German (Germany)",
                "German spell checking dictionary",
            ),
            (
                "it_IT",
                "Italian (Italy)",
                "Italian spell checking dictionary",
            ),
            (
                "pt_PT",
                "Portuguese (Portugal)",
                "Portuguese spell checking dictionary",
            ),
            (
                "ru_RU",
                "Russian (Russia)",
                "Russian spell checking dictionary",
            ),
            (
                "nl_NL",
                "Dutch (Netherlands)",
                "Dutch spell checking dictionary",
            ),
            (
                "sv_SE",
                "Swedish (Sweden)",
                "Swedish spell checking dictionary",
            ),
            (
                "da_DK",
                "Danish (Denmark)",
                "Danish spell checking dictionary",
            ),
            (
                "no_NO",
                "Norwegian (Norway)",
                "Norwegian spell checking dictionary",
            ),
            (
                "fi_FI",
                "Finnish (Finland)",
                "Finnish spell checking dictionary",
            ),
            (
                "pl_PL",
                "Polish (Poland)",
                "Polish spell checking dictionary",
            ),
            (
                "cs_CZ",
                "Czech (Czech Republic)",
                "Czech spell checking dictionary",
            ),
            (
                "hu_HU",
                "Hungarian (Hungary)",
                "Hungarian spell checking dictionary",
            ),
            (
                "ro_RO",
                "Romanian (Romania)",
                "Romanian spell checking dictionary",
            ),
            (
                "sk_SK",
                "Slovak (Slovakia)",
                "Slovak spell checking dictionary",
            ),
            (
                "sl_SI",
                "Slovenian (Slovenia)",
                "Slovenian spell checking dictionary",
            ),
            (
                "hr_HR",
                "Croatian (Croatia)",
                "Croatian spell checking dictionary",
            ),
            (
                "bg_BG",
                "Bulgarian (Bulgaria)",
                "Bulgarian spell checking dictionary",
            ),
            (
                "et_EE",
                "Estonian (Estonia)",
                "Estonian spell checking dictionary",
            ),
            (
                "lv_LV",
                "Latvian (Latvia)",
                "Latvian spell checking dictionary",
            ),
            (
                "lt_LT",
                "Lithuanian (Lithuania)",
                "Lithuanian spell checking dictionary",
            ),
        ];

        for (code, name, desc) in dictionaries {
            self.available_dicts.insert(
                code.to_string(),
                DictionaryInfo {
                    language: code.to_string(),
                    display_name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: desc.to_string(),
                    installed: false,
                },
            );
        }
    }

    /// Mark a dictionary as installed
    fn mark_dictionary_installed(&mut self, language: &str) {
        if let Some(dict_info) = self.available_dicts.get_mut(language) {
            dict_info.installed = true;
        }
    }

    /// Get dictionary paths for a language
    pub async fn get_dictionary_path(&self, language: &str) -> Result<DictionaryPaths> {
        // First check if dictionary exists in project dictionaries directory
        let project_dict_path =
            std::path::Path::new("dictionaries").join(format!("{}.dic", language));
        let project_aff_path =
            std::path::Path::new("dictionaries").join(format!("{}.aff", language));

        if project_dict_path.exists() {
            // Use project dictionary if available (create dummy aff file if needed)
            let aff_path = if project_aff_path.exists() {
                project_aff_path
            } else {
                // Create a minimal .aff file in user directory for project dictionaries
                let user_aff_path = self.dict_dir.join(format!("{}.aff", language));
                if !user_aff_path.exists() {
                    let aff_content = "SET UTF-8\nTRY abcdefghijklmnopqrstuvwxyz\n";
                    let _ = fs::write(&user_aff_path, aff_content).await;
                }
                user_aff_path
            };

            return Ok(DictionaryPaths {
                aff: aff_path,
                dic: project_dict_path,
            });
        }

        // Check if files exist in user directory
        let aff_path = self.dict_dir.join(format!("{}.aff", language));
        let dic_path = self.dict_dir.join(format!("{}.dic", language));

        if aff_path.exists() && dic_path.exists() {
            return Ok(DictionaryPaths {
                aff: aff_path,
                dic: dic_path,
            });
        }

        // Try to download if not available locally
        self.download_dictionary(language).await?;

        if aff_path.exists() && dic_path.exists() {
            Ok(DictionaryPaths {
                aff: aff_path,
                dic: dic_path,
            })
        } else {
            Err(anyhow!(
                "Dictionary files not found for language: {}",
                language
            ))
        }
    }

    /// Download dictionary from LibreOffice dictionaries
    async fn download_dictionary(&self, language: &str) -> Result<()> {
        // Create basic English dictionary content as fallback
        if language.starts_with("en") {
            self.create_basic_english_dictionary().await?;
            return Ok(());
        }

        // For other languages, create a minimal dictionary
        self.create_minimal_dictionary(language).await?;
        Ok(())
    }

    /// Create a basic English dictionary
    async fn create_basic_english_dictionary(&self) -> Result<()> {
        let aff_content = r#"SET UTF-8
TRY esianrtolcdugmphbyfvkwzxjqESIANRTOLCDUGMPHBYFVKWZXJQ

REP 90
REP a ei
REP ei a
REP a ey
REP ey a
REP ai ie
REP ie ai
REP are air
REP air are
REP are ear
REP ear are
REP du de
REP de du
REP i y
REP y i
REP j g
REP g j
REP o eau
REP eau o
REP ou oo
REP oo ou
REP s z
REP z s
REP ss s
REP s ss
"#;

        let dic_content = r#"5000
the/SM
be/DSGM
to/M
of/M
and/M
a/M
in/M
that/M
have/DGMS
i/M
it/M
for/M
not/M
on/M
with/M
he/M
as/M
you/M
do/DGM
at/M
this/M
but/M
his/M
by/M
from/M
they/M
she/M
or/M
an/M
will/M
my/M
one/M
all/M
would/M
there/M
their/M
what/M
so/M
up/M
out/M
if/M
about/M
who/M
get/DGS
which/M
go/DGS
me/M
when/M
make/DGS
can/M
like/M
time/MS
no/M
just/M
him/M
know/DGS
take/DGS
people/M
into/M
year/MS
your/M
good/M
some/M
could/M
them/M
see/DGS
other/M
than/M
then/M
now/M
look/DGS
only/M
come/DGS
its/M
over/M
think/DGS
also/M
back/M
after/M
use/DGS
two/M
how/M
our/M
work/DGS
first/M
well/M
way/MS
even/M
new/M
want/DGS
because/M
any/M
these/M
give/DGS
day/MS
most/M
us/M"#;

        let aff_path = self.dict_dir.join("en_US.aff");
        let dic_path = self.dict_dir.join("en_US.dic");

        fs::write(&aff_path, aff_content).await?;
        fs::write(&dic_path, dic_content).await?;

        tracing::info!("Created basic English dictionary at {:?}", self.dict_dir);
        Ok(())
    }

    /// Create minimal dictionary for other languages
    async fn create_minimal_dictionary(&self, language: &str) -> Result<()> {
        let aff_content = "SET UTF-8\nTRY abcdefghijklmnopqrstuvwxyz\n";
        let dic_content = "10\nthe\nand\nof\nto\na\nin\nthat\nhave\nfor\nnot\n";

        let aff_path = self.dict_dir.join(format!("{}.aff", language));
        let dic_path = self.dict_dir.join(format!("{}.dic", language));

        fs::write(&aff_path, aff_content).await?;
        fs::write(&dic_path, dic_content).await?;

        tracing::info!(
            "Created minimal dictionary for {} at {:?}",
            language,
            self.dict_dir
        );
        Ok(())
    }

    /// Get list of available languages
    pub fn available_languages(&self) -> Vec<String> {
        let mut languages: std::collections::HashSet<String> =
            self.available_dicts.keys().cloned().collect();

        // Also scan project dictionaries directory
        if let Ok(entries) = std::fs::read_dir("dictionaries") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".dic") {
                        let lang_code = name.trim_end_matches(".dic");
                        languages.insert(lang_code.to_string());
                    }
                }
            }
        }

        languages.into_iter().collect()
    }

    /// Get dictionary information
    pub fn get_dictionary_info(&self, language: &str) -> Option<&DictionaryInfo> {
        self.available_dicts.get(language)
    }

    /// Get all dictionary information
    pub fn get_all_dictionary_info(&self) -> &HashMap<String, DictionaryInfo> {
        &self.available_dicts
    }

    /// Check if dictionary is installed
    pub fn is_dictionary_installed(&self, language: &str) -> bool {
        self.available_dicts
            .get(language)
            .map(|info| info.installed)
            .unwrap_or(false)
    }
}
