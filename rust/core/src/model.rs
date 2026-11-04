use serde::{Deserialize, Serialize};

/// Gedeelde materialenbibliotheek: instellingen + per stuk + profielen.
/// Wordt los van projecten opgeslagen (bijv. materialen.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalogus {
    #[serde(default = "een")]
    pub versie: u32,
    #[serde(default = "standaard_instellingen")]
    pub instellingen: Instellingen,
    #[serde(default)]
    pub per_stuk: Vec<PerStuk>,
    #[serde(default)]
    pub profielen: Vec<Profiel>,
}

/// Eén project binnen het projectenbestand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectEntry {
    pub id: String,
    #[serde(default)]
    pub naam: String,
    #[serde(default)]
    pub regels: Vec<ProjectLijn>,
}

/// Projectenbestand: álle projecten in één JSON-bestand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectenData {
    #[serde(default = "een")]
    pub versie: u32,
    #[serde(default)]
    pub projecten: Vec<ProjectEntry>,
}

fn een() -> u32 {
    1
}

fn standaard_instellingen() -> Instellingen {
    Instellingen {
        zaagsnede_mm: 3.0,
        munt: "EUR".to_string(),
    }
}

impl Default for Catalogus {
    fn default() -> Self {
        Catalogus {
            versie: 1,
            instellingen: standaard_instellingen(),
            per_stuk: vec![
                PerStuk {
                    id: "s1".to_string(),
                    naam: "Schroef RVS 5 x 60 mm".to_string(),
                    prijs: 0.12,
                    info: "diameter 5 mm, lengte 60 mm".to_string(),
                    eenheid: "doos".to_string(),
                    aantal_per_eenheid: 100.0,
                    prijs_per_eenheid: 12.0,
                },
                PerStuk {
                    id: "s2".to_string(),
                    naam: "Slotbout M8 x 80 mm".to_string(),
                    prijs: 0.35,
                    info: "diameter 8 mm, lengte 80 mm, gegalvaniseerd".to_string(),
                    eenheid: "doos".to_string(),
                    aantal_per_eenheid: 50.0,
                    prijs_per_eenheid: 17.5,
                },
            ],
            profielen: vec![Profiel {
                id: "p1".to_string(),
                houtsoort: "Robinia".to_string(),
                dikte_mm: 45,
                breedte_mm: 120,
                inkoop: vec![InkoopOptie {
                    lengte_mm: 2000,
                    prijs: 48.50,
                }],
            }],
        }
    }
}

/// App-instellingen op schijf (werkmap waar materialen.json/projecten.json staan).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstellingen {
    #[serde(default)]
    pub werkmap: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instellingen {
    #[serde(default = "drie")]
    pub zaagsnede_mm: f64,
    #[serde(default = "eur")]
    pub munt: String,
}

fn drie() -> f64 {
    3.0
}
fn eur() -> String {
    "EUR".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerStuk {
    pub id: String,
    pub naam: String,
    /// Stukprijs — alleen gebruikt als er geen inkoopverpakking is opgegeven.
    #[serde(default)]
    pub prijs: f64,
    #[serde(default)]
    pub info: String,
    /// Inkoopeenheid, bijv. "doos" of "pak".
    #[serde(default)]
    pub eenheid: String,
    /// Aantal stuks per inkoopeenheid, bijv. 100.
    #[serde(default)]
    pub aantal_per_eenheid: f64,
    /// Totaalprijs per inkoopeenheid (prijs van de verpakking).
    #[serde(default)]
    pub prijs_per_eenheid: f64,
}

impl PerStuk {
    /// Stukprijs: automatisch berekend als prijs_verpakking / aantal_per_eenheid.
    pub fn stukprijs(&self) -> f64 {
        if self.aantal_per_eenheid > 0.0 {
            self.prijs_per_eenheid / self.aantal_per_eenheid
        } else {
            self.prijs
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profiel {
    pub id: String,
    #[serde(default)]
    pub houtsoort: String,
    #[serde(default)]
    pub dikte_mm: i64,
    #[serde(default)]
    pub breedte_mm: i64,
    #[serde(default)]
    pub inkoop: Vec<InkoopOptie>,
}

impl Profiel {
    pub fn naam(&self) -> String {
        format!("{} {} x {} mm", self.houtsoort, self.dikte_mm, self.breedte_mm)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkoopOptie {
    pub lengte_mm: i64,
    #[serde(default)]
    pub prijs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProjectLijn {
    Zaagdeel {
        id: String,
        #[serde(default)]
        profiel_id: Option<String>,
        #[serde(default)]
        lengte_mm: i64,
        #[serde(default)]
        aantal: u32,
    },
    PerStuk {
        id: String,
        #[serde(default)]
        materiaal_id: Option<String>,
        #[serde(default)]
        aantal: u32,
    },
}

impl ProjectLijn {
    pub fn id(&self) -> &str {
        match self {
            ProjectLijn::Zaagdeel { id, .. } => id,
            ProjectLijn::PerStuk { id, .. } => id,
        }
    }
}
