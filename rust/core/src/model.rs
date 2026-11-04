use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub versie: u32,
    #[serde(default = "standaard_instellingen")]
    pub instellingen: Instellingen,
    #[serde(default)]
    pub per_stuk: Vec<PerStuk>,
    #[serde(default)]
    pub profielen: Vec<Profiel>,
    #[serde(default)]
    pub project: Vec<ProjectLijn>,
}

fn standaard_instellingen() -> Instellingen {
    Instellingen {
        zaagsnede_mm: 3.0,
        btw_pct: 21.0,
        munt: "EUR".to_string(),
    }
}

impl Default for Data {
    fn default() -> Self {
        Data {
            versie: 1,
            instellingen: standaard_instellingen(),
            per_stuk: vec![
                PerStuk {
                    id: "s1".to_string(),
                    naam: "Schroef RVS 5 x 60 mm".to_string(),
                    prijs: 0.12,
                    info: "diameter 5 mm, lengte 60 mm".to_string(),
                },
                PerStuk {
                    id: "s2".to_string(),
                    naam: "Slotbout M8 x 80 mm".to_string(),
                    prijs: 0.35,
                    info: "diameter 8 mm, lengte 80 mm, gegalvaniseerd".to_string(),
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
            project: vec![
                ProjectLijn::Zaagdeel {
                    id: "l1".to_string(),
                    profiel_id: Some("p1".to_string()),
                    lengte_mm: 1000,
                    aantal: 2,
                },
                ProjectLijn::PerStuk {
                    id: "l2".to_string(),
                    materiaal_id: Some("s1".to_string()),
                    aantal: 24,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instellingen {
    #[serde(default = "drie")]
    pub zaagsnede_mm: f64,
    #[serde(default = "eenentwintig")]
    pub btw_pct: f64,
    #[serde(default = "eur")]
    pub munt: String,
}

fn drie() -> f64 {
    3.0
}
fn eenentwintig() -> f64 {
    21.0
}
fn eur() -> String {
    "EUR".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerStuk {
    pub id: String,
    pub naam: String,
    #[serde(default)]
    pub prijs: f64,
    #[serde(default)]
    pub info: String,
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
