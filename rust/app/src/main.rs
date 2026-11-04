use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use eframe::egui::Widget;
use speeltoestel_core::model::{Catalogus, InkoopOptie, PerStuk, Profiel, ProjectEntry, ProjectLijn, ProjectenData};
use speeltoestel_core::optimizer::{bereken_project, Berekening};

static ID_TELLER: AtomicU64 = AtomicU64::new(0);

const ACCENT: egui::Color32 = egui::Color32::from_rgb(0xA0, 0x62, 0x2D);
const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(0xF0, 0xEA, 0xE2);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(0xFB, 0xF9, 0xF6);
const CARD_BG: egui::Color32 = egui::Color32::WHITE;
const CARD_STROKE: egui::Color32 = egui::Color32::from_rgb(0xE3, 0xDC, 0xD2);
const DANGER: egui::Color32 = egui::Color32::from_rgb(0xB3, 0x26, 0x1E);

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(egui::Color32::WHITE))
            .fill(ACCENT)
            .min_size(egui::vec2(0.0, 32.0)),
    )
}

fn danger_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(DANGER))
            .min_size(egui::vec2(0.0, 32.0)),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn datum_vandaag() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let mut year = 1970i64;
    let mut rest = days;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let yd = if leap { 366 } else { 365 };
        if rest >= yd {
            year += 1;
            rest -= yd;
        } else {
            break;
        }
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    for m in mdays {
        if rest >= m {
            rest -= m;
            month += 1;
        } else {
            break;
        }
    }
    format!("{:04}-{:02}-{:02}", year, month, rest + 1)
}

fn fmt_bedrag_munt(munt: &str, x: f64) -> String {
    let symbool = if munt == "EUR" { "€" } else { munt };
    format!("{symbool} {:.2}", x)
}

fn text_field(ui: &mut egui::Ui, text: &mut String, width: f32) -> egui::Response {
    ui.add_sized(egui::vec2(width, 32.0), egui::TextEdit::singleline(text))
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::default()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0_f32, CARD_STROKE))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
    ui.add_space(16.0);
}

fn nieuw_id() -> String {
    let n = ID_TELLER.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("id-{nanos}-{n}")
}

#[derive(PartialEq, Clone, Copy)]
enum MatTab {
    PerStuk,
    Halffabricaten,
    Instellingen,
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Projecten,
    Materialen,
    Project,
    Calculatie,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Projecten => "Projecten",
            Tab::Materialen => "Materialen",
            Tab::Project => "Project",
            Tab::Calculatie => "Calculatie",
        }
    }
}

struct App {
    catalogus: Catalogus,
    projecten: Vec<ProjectEntry>,
    huidig_project: Option<String>,
    werkmap: Option<PathBuf>,
    setup_nodig: bool,
    map_keuze: Option<PathBuf>,
    last_saved: String,
    tab: Tab,
    mat_tab: MatTab,
    status: String,
    nieuw_per_stuk: (String, String, String, String, String),
    nieuw_profiel_inkoop: (String, String),
    nieuwe_project_naam: String,
}

impl Default for App {
    fn default() -> Self {
        App {
            catalogus: Catalogus::default(),
            projecten: vec![ProjectEntry {
                id: "pr1".to_string(),
                naam: "Voorbeeldproject".to_string(),
                regels: vec![
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
            }],
            huidig_project: Some("pr1".to_string()),
            werkmap: None,
            setup_nodig: false,
            map_keuze: None,
            last_saved: String::new(),
            tab: Tab::Projecten,
            mat_tab: MatTab::PerStuk,
            status: "Onopgeslagen wijzigingen".to_string(),
            nieuw_per_stuk: (String::new(), String::new(), String::new(), String::new(), String::new()),
            nieuw_profiel_inkoop: (String::new(), String::new()),
            nieuwe_project_naam: String::new(),
        }
    }
}

impl App {
    fn fmt_bedrag(&self, x: f64) -> String {
        let munt = &self.catalogus.instellingen.munt;
        let symbool = if munt == "EUR" { "€" } else { munt.as_str() };
        format!("{symbool} {:.2}", x)
    }

    fn parse_getal(s: &str) -> f64 {
        s.replace(',', ".").parse().unwrap_or(0.0)
    }

    fn set_status(&mut self) {
        self.status = match &self.werkmap {
            Some(map) => format!(
                "Werkmap: {} — alles wordt automatisch opgeslagen (materialen.json + projecten.json)",
                map.display()
            ),
            None => "Kies eerst een werkmap…".to_string(),
        };
    }

    fn app_instellingen_pad() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        if cfg!(target_os = "macos") {
            PathBuf::from(home).join("Library/Application Support/speeltoestel-calculator/settings.json")
        } else if cfg!(target_os = "windows") {
            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| home.clone());
            PathBuf::from(appdata).join("speeltoestel-calculator/settings.json")
        } else {
            PathBuf::from(home).join(".config/speeltoestel-calculator/settings.json")
        }
    }

    fn materialen_pad(&self) -> Option<PathBuf> {
        self.werkmap.as_ref().map(|m| m.join("materialen.json"))
    }

    fn projecten_pad(&self) -> Option<PathBuf> {
        self.werkmap.as_ref().map(|m| m.join("projecten.json"))
    }

    fn init_bij_opstarten(&mut self) {
        let pad = Self::app_instellingen_pad();
        match std::fs::read_to_string(&pad) {
            Ok(tekst) => match serde_json::from_str::<speeltoestel_core::model::AppInstellingen>(&tekst) {
                Ok(inst) if !inst.werkmap.is_empty() => {
                    self.werkmap = Some(PathBuf::from(&inst.werkmap));
                    self.laad_uit_werkmap();
                }
                _ => self.setup_nodig = true,
            },
            Err(_) => self.setup_nodig = true,
        }
        self.set_status();
    }

    fn laad_uit_werkmap(&mut self) {
        if let Some(mp) = self.materialen_pad() {
            match std::fs::read_to_string(&mp) {
                Ok(tekst) => {
                    if let Ok(c) = serde_json::from_str::<Catalogus>(&tekst) {
                        self.catalogus = c;
                    }
                }
                Err(_) => {
                    let _ = std::fs::write(&mp, serde_json::to_string_pretty(&Catalogus::default()).unwrap_or_default());
                }
            }
        }
        if let Some(pp) = self.projecten_pad() {
            match std::fs::read_to_string(&pp) {
                Ok(tekst) => {
                    let value: serde_json::Value = match serde_json::from_str(&tekst) {
                        Ok(v) => v,
                        Err(_) => return,
                    };
                    if value.get("projecten").is_some() {
                        if let Ok(data) = serde_json::from_value::<ProjectenData>(value) {
                            self.projecten = data.projecten;
                        }
                    } else if value.get("project").is_some() {
                        if let Ok(regels) = serde_json::from_value::<Vec<ProjectLijn>>(value["project"].clone()) {
                            self.projecten = vec![ProjectEntry {
                                id: nieuw_id(),
                                naam: "Geïmporteerd project".to_string(),
                                regels,
                            }];
                        }
                    }
                    self.huidig_project = self.projecten.first().map(|p| p.id.clone());
                }
                Err(_) => {
                    let data = ProjectenData {
                        versie: 1,
                        projecten: std::mem::take(&mut self.projecten),
                    };
                    let _ = std::fs::write(&pp, serde_json::to_string_pretty(&data).unwrap_or_default());
                    self.projecten = data.projecten;
                }
            }
        }
        self.last_saved = self.huidige_state_json();
    }

    fn huidige_state_json(&self) -> String {
        let cat = serde_json::to_string(&self.catalogus).unwrap_or_default();
        let proj = serde_json::to_string(&ProjectenData {
            versie: 1,
            projecten: self.projecten.clone(),
        })
        .unwrap_or_default();
        format!("{cat}\n{proj}")
    }

    fn kies_werkmap(&mut self) {
        if let Some(map) = rfd::FileDialog::new().pick_folder() {
            self.map_keuze = Some(map);
        }
    }

    fn bevestig_werkmap(&mut self) {
        if let Some(map) = self.map_keuze.clone() {
            let inst = speeltoestel_core::model::AppInstellingen {
                werkmap: map.display().to_string(),
            };
            if let Some(pad) = Self::app_instellingen_pad().parent().map(|p| p.to_path_buf()) {
                let _ = std::fs::create_dir_all(&pad);
            }
            if let Ok(json) = serde_json::to_string_pretty(&inst) {
                let _ = std::fs::write(Self::app_instellingen_pad(), json);
            }
            self.werkmap = Some(map);
            self.setup_nodig = false;
            self.laad_uit_werkmap();
            self.set_status();
        }
    }

    fn autosave(&mut self) {
        if self.werkmap.is_none() {
            return;
        }
        let state = self.huidige_state_json();
        if state == self.last_saved {
            return;
        }
        self.forceer_opslaan();
        self.last_saved = state;
        self.status = format!(
            "Automatisch opgeslagen om {} — {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| format!("{}s", d.as_secs()))
                .unwrap_or_default(),
            self.werkmap.as_ref().map(|m| m.display().to_string()).unwrap_or_default()
        );
    }

    fn forceer_opslaan(&mut self) {
        if let Some(mp) = self.materialen_pad() {
            if let Ok(json) = serde_json::to_string_pretty(&self.catalogus) {
                if std::fs::write(&mp, json).is_ok() {
                    self.last_saved = self.huidige_state_json();
                }
            }
        }
        if let Some(pp) = self.projecten_pad() {
            let data = ProjectenData {
                versie: 1,
                projecten: self.projecten.clone(),
            };
            if let Ok(json) = serde_json::to_string_pretty(&data) {
                let _ = std::fs::write(&pp, json);
            }
        }
    }
    fn huidig_project(&self) -> Option<&ProjectEntry> {
        self.huidig_project
            .as_ref()
            .and_then(|id| self.projecten.iter().find(|p| &p.id == id))
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .frame(
                egui::Frame::default()
                    .fill(SIDEBAR_BG)
                    .inner_margin(egui::Margin::same(14.0)),
            )
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Speeltoestel\nCalculator")
                        .strong()
                        .size(20.0)
                        .color(ACCENT),
                );
                ui.add_space(18.0);
                for tab in [Tab::Projecten, Tab::Materialen, Tab::Project, Tab::Calculatie] {
                    let selected = self.tab == tab;
                    let response = ui.add_sized(
                        egui::vec2(ui.available_width(), 38.0),
                        egui::SelectableLabel::new(selected, egui::RichText::new(tab.label()).size(15.0)),
                    );
                    if response.clicked() {
                        self.tab = tab;
                    }
                    ui.add_space(4.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(egui::RichText::new(&self.status).weak().small());
                });
            });
    }

    fn topbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.tab.label()).strong().size(16.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Werkmap wijzigen…").clicked() {
                    self.kies_werkmap();
                    if self.map_keuze.is_some() {
                        self.bevestig_werkmap();
                    }
                }
                ui.label(egui::RichText::new(&self.status).weak().small());
            });
        });
        ui.separator();
    }

    fn setup_modal(&mut self, ctx: &egui::Context) {
        egui::Window::new("Welkom bij de Speeltoestel Calculator")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.label("Kies de map waarin je materialen en projecten worden bewaard.");
                ui.label("Daarin staan twee bestanden: materialen.json en projecten.json.");
                ui.label("Alles wat je wijzigt wordt daarna automatisch opgeslagen in deze map.");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Kies map…").clicked() {
                        self.kies_werkmap();
                    }
                    if let Some(map) = &self.map_keuze {
                        ui.label(egui::RichText::new(map.display().to_string()).small());
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let ok = primary_button(ui, "Opslaan en doorgaan");
                    if ok.clicked() && self.map_keuze.is_some() {
                        self.bevestig_werkmap();
                    }
                });
            });
    }

    fn tab_projecten(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
        ui.heading("Projecten");
        ui.label("Alle projecten staan in één bestand. Open een project om de onderdelen te bewerken.");
        ui.add_space(6.0);

        let kosten: Vec<f64> = self
            .projecten
            .iter()
            .map(|p| bereken_project(&self.catalogus, &p.regels).totaal_excl)
            .collect();

        let mut actie: Option<(&'static str, String)> = None;
        egui::Grid::new("projecten_grid")
            .striped(true).spacing([14.0, 10.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Naam").strong());
                ui.label(egui::RichText::new("Onderdelen").strong());
                ui.label(egui::RichText::new("Totaal excl. BTW").strong());
                ui.label("");
                ui.label("");
                ui.end_row();
                for i in 0..self.projecten.len() {
                    let id = self.projecten[i].id.clone();
                    let is_huidig = self.huidig_project.as_deref() == Some(id.as_str());
                    let mut naam = self.projecten[i].naam.clone();
                    text_field(ui, &mut naam, 320.0);
                    if naam != self.projecten[i].naam {
                        self.projecten[i].naam = naam;
                    }
                    ui.label(format!("{}", self.projecten[i].regels.len()));
                    let mut kost_label = self.fmt_bedrag(kosten[i]);
                    if is_huidig {
                        kost_label.push_str("  (actief)");
                    }
                    ui.label(kost_label);
                    if primary_button(ui, "Openen").clicked() {
                        actie = Some(("openen", id.clone()));
                    }
                    if danger_button(ui, "Verwijder").clicked() {
                        actie = Some(("verwijder", id));
                    }
                    ui.end_row();
                }
            });
        match actie {
            Some(("openen", id)) => {
                self.huidig_project = Some(id);
                self.tab = Tab::Project;
            }
            Some(("verwijder", id)) => {
                self.projecten.retain(|p| p.id != id);
                if self.huidig_project.as_deref() == Some(id.as_str()) {
                    self.huidig_project = None;
                }
            }
            _ => {}
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            egui::TextEdit::singleline(&mut self.nieuwe_project_naam)
                .desired_width(320.0)
                .hint_text("Naam nieuw project")
                .ui(ui);
            if primary_button(ui, "+ Nieuw project").clicked() {
                let naam = if self.nieuwe_project_naam.trim().is_empty() {
                    format!("Project {}", self.projecten.len() + 1)
                } else {
                    self.nieuwe_project_naam.trim().to_string()
                };
                let id = nieuw_id();
                self.projecten.push(ProjectEntry {
                    id: id.clone(),
                    naam,
                    regels: Vec::new(),
                });
                self.huidig_project = Some(id);
                self.nieuwe_project_naam.clear();
                self.tab = Tab::Project;
            }
        });
        });
    }

    fn tab_materialen(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (t, label) in [
                (MatTab::PerStuk, "Producten per stuk"),
                (MatTab::Halffabricaten, "Half-fabricaten"),
                (MatTab::Instellingen, "Instellingen"),
            ] {
                let selected = self.mat_tab == t;
                if ui
                    .selectable_label(selected, egui::RichText::new(label).size(14.0))
                    .clicked()
                {
                    self.mat_tab = t;
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| match self.mat_tab {
            MatTab::PerStuk => self.mat_per_stuk(ui),
            MatTab::Halffabricaten => self.mat_halffabricaten(ui),
            MatTab::Instellingen => self.mat_instellingen(ui),
        });
    }

    fn mat_instellingen(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.heading("Instellingen");
            ui.horizontal(|ui| {
                ui.label("Zaagsnede (mm):");
                egui::DragValue::new(&mut self.catalogus.instellingen.zaagsnede_mm)
                    .speed(0.5)
                    .range(0.0..=20.0)
                    .ui(ui);
                ui.label("Munt:");
                text_field(ui, &mut self.catalogus.instellingen.munt, 80.0);
            });
        });
    }

    fn mat_per_stuk(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.heading("Producten per stuk");
            ui.label("Schroeven, bouten, etc. Geef per product de inkoopverpakking op (eenheid + aantal + prijs per verpakking); de stukprijs wordt automatisch berekend. Excl. BTW.");
            egui::Grid::new("per_stuk_grid").striped(true).spacing([14.0, 10.0]).show(ui, |ui| {
                ui.label(egui::RichText::new("Naam").strong());
                ui.label(egui::RichText::new("Stukprijs (auto)").strong());
                ui.label(egui::RichText::new("Eenheid").strong());
                ui.label(egui::RichText::new("Aantal/eenh.").strong());
                ui.label(egui::RichText::new("Prijs/eenh.").strong());
                ui.label(egui::RichText::new("Extra info").strong());
                ui.label("");
                ui.end_row();
                let mut verwijder: Option<String> = None;
                let munt = self.catalogus.instellingen.munt.clone();
                for m in self.catalogus.per_stuk.iter_mut() {
                    text_field(ui, &mut m.naam, 200.0);
                    ui.label(egui::RichText::new(fmt_bedrag_munt(&munt, m.stukprijs())).weak());
                    text_field(ui, &mut m.eenheid, 90.0);
                    egui::DragValue::new(&mut m.aantal_per_eenheid)
                        .speed(1.0)
                        .range(0.0..=1000000.0)
                        .ui(ui);
                    egui::DragValue::new(&mut m.prijs_per_eenheid)
                        .speed(0.5)
                        .range(0.0..=1000000.0)
                        .ui(ui);
                    text_field(ui, &mut m.info, 180.0);
                    if danger_button(ui, "Verwijder").clicked() {
                        verwijder = Some(m.id.clone());
                    }
                    ui.end_row();
                }
                if let Some(id) = verwijder {
                    self.catalogus.per_stuk.retain(|m| m.id != id);
                    for p in self.projecten.iter_mut() {
                        p.regels.retain(|l| match l {
                            ProjectLijn::PerStuk { materiaal_id, .. } => {
                                materiaal_id.as_deref() != Some(id.as_str())
                            }
                            _ => true,
                        });
                    }
                }
                // invoerregel: zelfde kolommen/breedtes als de tabelrijen
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.0)
                    .desired_width(200.0)
                    .hint_text("Naam")
                    .ui(ui);
                ui.label("");
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.1)
                    .desired_width(90.0)
                    .hint_text("Eenheid")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.2)
                    .desired_width(90.0)
                    .hint_text("Aantal/eenh.")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.3)
                    .desired_width(100.0)
                    .hint_text("Prijs/eenh.")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.4)
                    .desired_width(180.0)
                    .hint_text("Extra info")
                    .ui(ui);
                if primary_button(ui, "Toevoegen").clicked() {
                    if !self.nieuw_per_stuk.0.trim().is_empty() {
                        let aantal_per_eenheid = Self::parse_getal(&self.nieuw_per_stuk.2);
                        let prijs_per_eenheid = Self::parse_getal(&self.nieuw_per_stuk.3);
                        let stukprijs = if aantal_per_eenheid > 0.0 {
                            prijs_per_eenheid / aantal_per_eenheid
                        } else {
                            prijs_per_eenheid
                        };
                        self.catalogus.per_stuk.push(PerStuk {
                            id: nieuw_id(),
                            naam: self.nieuw_per_stuk.0.trim().to_string(),
                            prijs: stukprijs,
                            info: self.nieuw_per_stuk.4.trim().to_string(),
                            eenheid: self.nieuw_per_stuk.1.trim().to_string(),
                            aantal_per_eenheid,
                            prijs_per_eenheid,
                        });
                        self.nieuw_per_stuk = (String::new(), String::new(), String::new(), String::new(), String::new());
                    }
                }
                ui.end_row();
            });
        });
    }

    fn mat_halffabricaten(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.heading("Half-fabricaten (zaagdelen)");
            ui.label("Planken, palen, etc. Per profiel meerdere inkooplengtes mogelijk; de app kiest de goedkoopste samenstelling.");
            let mut verwijder_profiel: Option<String> = None;
            let mut verwijder_inkoop: Option<(String, usize)> = None;
            let profielen_len = self.catalogus.profielen.len();
            for pi in 0..profielen_len {
                let profiel_id = self.catalogus.profielen[pi].id.clone();
                egui::CollapsingHeader::new(format!("Profiel: {}", self.catalogus.profielen[pi].naam()))
                    .id_salt(&profiel_id)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Houtsoort:");
                            text_field(ui, &mut self.catalogus.profielen[pi].houtsoort, 180.0);
                            ui.label("Dikte (mm):");
                            egui::DragValue::new(&mut self.catalogus.profielen[pi].dikte_mm)
                                .speed(1.0)
                                .range(1..=500)
                                .ui(ui);
                            ui.label("Breedte (mm):");
                            egui::DragValue::new(&mut self.catalogus.profielen[pi].breedte_mm)
                                .speed(1.0)
                                .range(1..=1000)
                                .ui(ui);
                            if danger_button(ui, "Profiel verwijderen").clicked() {
                                verwijder_profiel = Some(profiel_id.clone());
                            }
                        });
                        ui.label("Inkooplengtes en prijzen (exclusief BTW):");
                        egui::Grid::new(format!("inkoop_{profiel_id}")).show(ui, |ui| {
                            ui.label(egui::RichText::new("Lengte (mm)").strong());
                            ui.label(egui::RichText::new("Prijs/stuk").strong());
                            ui.label("");
                            ui.end_row();
                            let inkoop_len = self.catalogus.profielen[pi].inkoop.len();
                            for oi in 0..inkoop_len {
                                egui::DragValue::new(&mut self.catalogus.profielen[pi].inkoop[oi].lengte_mm)
                                    .speed(10.0)
                                    .range(1..=100000)
                                    .ui(ui);
                                egui::DragValue::new(&mut self.catalogus.profielen[pi].inkoop[oi].prijs)
                                    .speed(0.5)
                                    .range(0.0..=1000000.0)
                                    .ui(ui);
                                if danger_button(ui, "Verwijder").clicked() {
                                    verwijder_inkoop = Some((profiel_id.clone(), oi));
                                }
                                ui.end_row();
                            }
                        });
                        ui.horizontal(|ui| {
                            egui::TextEdit::singleline(&mut self.nieuw_profiel_inkoop.0)
                                .desired_width(130.0)
                                .hint_text("Lengte (mm)")
                                .ui(ui);
                            egui::TextEdit::singleline(&mut self.nieuw_profiel_inkoop.1)
                                .desired_width(120.0)
                                .hint_text("Prijs")
                                .ui(ui);
                            if primary_button(ui, "Inkooplengte toevoegen").clicked() {
                                let lengte = Self::parse_getal(&self.nieuw_profiel_inkoop.0) as i64;
                                let prijs = Self::parse_getal(&self.nieuw_profiel_inkoop.1);
                                if lengte > 0 {
                                    self.catalogus.profielen[pi].inkoop.push(InkoopOptie {
                                        lengte_mm: lengte,
                                        prijs,
                                    });
                                    self.nieuw_profiel_inkoop = (String::new(), String::new());
                                }
                            }
                        });
                    });
            }
            if let Some(id) = verwijder_profiel {
                self.catalogus.profielen.retain(|p| p.id != id);
                for p in self.projecten.iter_mut() {
                    p.regels.retain(|l| match l {
                        ProjectLijn::Zaagdeel { profiel_id, .. } => {
                            profiel_id.as_deref() != Some(id.as_str())
                        }
                        _ => true,
                    });
                }
            }
            if let Some((pid, oi)) = verwijder_inkoop {
                if let Some(p) = self.catalogus.profielen.iter_mut().find(|p| p.id == pid) {
                    if oi < p.inkoop.len() {
                        p.inkoop.remove(oi);
                    }
                }
            }
            if primary_button(ui, "+ Profiel toevoegen").clicked() {
                self.catalogus.profielen.push(Profiel {
                    id: nieuw_id(),
                    houtsoort: "Houtsoort".to_string(),
                    dikte_mm: 45,
                    breedte_mm: 120,
                    inkoop: vec![InkoopOptie {
                        lengte_mm: 2000,
                        prijs: 0.0,
                    }],
                });
            }
        });
    }

    fn regels_editor(catalogus: &Catalogus, ui: &mut egui::Ui, regels: &mut Vec<ProjectLijn>) {
        egui::Grid::new("project_grid").striped(true).spacing([14.0, 10.0]).show(ui, |ui| {
            ui.label(egui::RichText::new("Type").strong());
            ui.label(egui::RichText::new("Materiaal / profiel").strong());
            ui.label(egui::RichText::new("Lengte (mm)").strong());
            ui.label(egui::RichText::new("Aantal").strong());
            ui.label("");
            ui.end_row();
            let mut verwijder: Option<String> = None;
            let len = regels.len();
            for i in 0..len {
                let lijn_id = regels[i].id().to_string();
                let is_zaagdeel = matches!(regels[i], ProjectLijn::Zaagdeel { .. });
                let mut nieuw_type_zaagdeel = is_zaagdeel;
                egui::ComboBox::from_id_salt(format!("type_{lijn_id}")).width(150.0)
                    .selected_text(if is_zaagdeel { "Zaagdeel" } else { "Per stuk" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut nieuw_type_zaagdeel, true, "Zaagdeel");
                        ui.selectable_value(&mut nieuw_type_zaagdeel, false, "Per stuk");
                    });
                if nieuw_type_zaagdeel != is_zaagdeel {
                    let id = lijn_id.clone();
                    if let Some(l) = regels.iter_mut().find(|l| l.id() == id) {
                        *l = if nieuw_type_zaagdeel {
                            ProjectLijn::Zaagdeel {
                                id,
                                profiel_id: catalogus.profielen.first().map(|p| p.id.clone()),
                                lengte_mm: 0,
                                aantal: 1,
                            }
                        } else {
                            ProjectLijn::PerStuk {
                                id,
                                materiaal_id: catalogus.per_stuk.first().map(|m| m.id.clone()),
                                aantal: 1,
                            }
                        };
                    }
                }
                match &mut regels[i] {
                    ProjectLijn::PerStuk {
                        materiaal_id, aantal, ..
                    } => {
                        let mut gekozen = materiaal_id.clone().unwrap_or_default();
                        egui::ComboBox::from_id_salt(format!("mat_{lijn_id}")).width(300.0)
                            .selected_text(
                                catalogus
                                    .per_stuk
                                    .iter()
                                    .find(|m| Some(&m.id) == materiaal_id.as_ref())
                                    .map(|m| m.naam.clone())
                                    .unwrap_or_else(|| "– kies –".to_string()),
                            )
                            .show_ui(ui, |ui| {
                                for m in &catalogus.per_stuk {
                                    ui.selectable_value(&mut gekozen, m.id.clone(), &m.naam);
                                }
                            });
                        if gekozen != materiaal_id.as_deref().unwrap_or("")
                            || (materiaal_id.is_none() && !gekozen.is_empty())
                        {
                            *materiaal_id = Some(gekozen);
                        }
                        ui.label("–");
                        egui::DragValue::new(aantal).speed(1.0).range(0..=100000).ui(ui);
                    }
                    ProjectLijn::Zaagdeel {
                        profiel_id,
                        lengte_mm,
                        aantal,
                        ..
                    } => {
                        let mut gekozen = profiel_id.clone().unwrap_or_default();
                        egui::ComboBox::from_id_salt(format!("prof_{lijn_id}")).width(300.0)
                            .selected_text(
                                catalogus
                                    .profielen
                                    .iter()
                                    .find(|p| Some(&p.id) == profiel_id.as_ref())
                                    .map(|p| p.naam())
                                    .unwrap_or_else(|| "– kies –".to_string()),
                            )
                            .show_ui(ui, |ui| {
                                for p in &catalogus.profielen {
                                    ui.selectable_value(&mut gekozen, p.id.clone(), p.naam());
                                }
                            });
                        if gekozen != profiel_id.as_deref().unwrap_or("")
                            || (profiel_id.is_none() && !gekozen.is_empty())
                        {
                            *profiel_id = Some(gekozen);
                        }
                        egui::DragValue::new(lengte_mm)
                            .speed(10.0)
                            .range(0..=100000)
                            .ui(ui);
                        egui::DragValue::new(aantal).speed(1.0).range(0..=100000).ui(ui);
                    }
                }
                if danger_button(ui, "Verwijder").clicked() {
                    verwijder = Some(lijn_id);
                }
                ui.end_row();
            }
            if let Some(id) = verwijder {
                regels.retain(|l| l.id() != id);
            }
        });
        ui.horizontal(|ui| {
            if primary_button(ui, "+ Per stuk toevoegen").clicked() {
                regels.push(ProjectLijn::PerStuk {
                    id: nieuw_id(),
                    materiaal_id: catalogus.per_stuk.first().map(|m| m.id.clone()),
                    aantal: 1,
                });
            }
            if primary_button(ui, "+ Zaagdeel toevoegen").clicked() {
                regels.push(ProjectLijn::Zaagdeel {
                    id: nieuw_id(),
                    profiel_id: catalogus.profielen.first().map(|p| p.id.clone()),
                    lengte_mm: 0,
                    aantal: 1,
                });
            }
        });
    }

    fn tab_project(&mut self, ui: &mut egui::Ui) {
        let Some(pos) = self
            .huidig_project
            .as_ref()
            .and_then(|id| self.projecten.iter().position(|p| &p.id == id))
        else {
            ui.heading("Project");
            ui.label("Geen project geopend. Kies een project in het tabblad Projecten of maak een nieuw project aan.");
            if ui.button("Naar Projecten").clicked() {
                self.tab = Tab::Projecten;
            }
            return;
        };
        let naam = self.projecten[pos].naam.clone();
        ui.heading(format!("Project: {naam}"));
        ui.label("Benodigde onderdelen voor dit project.");
        ui.add_space(4.0);
        card(ui, |ui| {
            let regels = &mut self.projecten[pos].regels;
            Self::regels_editor(&self.catalogus, ui, regels);
        });
    }

    fn bom_html(&self, project_naam: &str, res: &Berekening) -> String {
        let mut bom_rows = String::new();
        for zr in &res.zaag_resultaten {
            match &zr.resultaat {
                Some(r) => {
                    for (lengte, prijs, aantal, bedrag) in &r.per_lengte {
                        bom_rows.push_str(&format!(
                            "<tr><td>{}</td><td>{} mm</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                            html_escape(&zr.profiel.naam()),
                            lengte,
                            aantal,
                            self.fmt_bedrag(*prijs),
                            self.fmt_bedrag(*bedrag)
                        ));
                    }
                }
                None => {
                    bom_rows.push_str(&format!(
                        "<tr><td>{}</td><td colspan=\"4\" style=\"color:#B3261E\">geen passende inkooplengte</td></tr>",
                        html_escape(&zr.profiel.naam())
                    ));
                }
            }
        }
        for l in &res.per_stuk {
            let spec = if l.eenheid.is_empty() {
                format!("per stuk (nodig {})", l.aantal)
            } else {
                format!(
                    "nodig {} stuks · per {} {}",
                    l.aantal, l.aantal_per_eenheid, l.eenheid
                )
            };
            let aantal = if l.eenheid.is_empty() {
                l.inkoop_aantal.to_string()
            } else {
                format!("{} {}", l.inkoop_aantal, l.eenheid)
            };
            let prijs = if l.eenheid.is_empty() {
                self.fmt_bedrag(l.prijs_per_stuk)
            } else {
                self.fmt_bedrag(l.prijs_per_eenheid)
            };
            bom_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&l.naam),
                html_escape(&spec),
                aantal,
                prijs,
                self.fmt_bedrag(l.kosten)
            ));
        }

        let mut zaag_rows = String::new();
        for zr in &res.zaag_resultaten {
            if let Some(r) = &zr.resultaat {
                for (i, bak) in r.bakken.iter().enumerate() {
                    let stukken: Vec<String> = bak.stukken.iter().map(|s| format!("{s} mm")).collect();
                    zaag_rows.push_str(&format!(
                        "<tr><td>{}</td><td>{} ({} mm)</td><td>{}</td><td>{} mm</td></tr>",
                        html_escape(&zr.profiel.naam()),
                        i + 1,
                        bak.lengte,
                        html_escape(&stukken.join(" + ")),
                        bak.rest
                    ));
                }
            }
        }
        if zaag_rows.is_empty() {
            zaag_rows = "<tr><td colspan=\"4\">Geen zaagdelen in dit project.</td></tr>".to_string();
        }

        format!(
            r#"<!DOCTYPE html>
<html lang="nl">
<head>
<meta charset="utf-8">
<title>Bill of Materials — {naam}</title>
<style>
body {{ font-family: -apple-system, "Segoe UI", Arial, sans-serif; margin: 32px; color: #111; }}
h1 {{ font-size: 22px; color: #A0622D; }}
h2 {{ font-size: 17px; margin-top: 28px; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 8px; }}
th, td {{ border: 1px solid #ccc; padding: 6px 10px; text-align: left; }}
th {{ background: #F0EAE2; }}
.totaal td {{ font-weight: bold; font-size: 17px; color: #A0622D; border-top: 2px solid #A0622D; }}
button {{ font-size: 15px; padding: 8px 16px; background: #A0622D; color: #fff; border: none; border-radius: 8px; cursor: pointer; }}
.meta {{ color: #666; font-size: 13px; }}
@media print {{ button {{ display: none; }} body {{ margin: 0; }} }}
</style>
</head>
<body>
<button onclick="window.print()">Afdrukken</button>
<h1>Bill of Materials — {naam}</h1>
<p class="meta">Gegenereerd op {datum} · Zaagsnede: {zaagsnede} mm per zaagdeel · Alle prijzen excl. BTW</p>
<h2>Aan te schaffen</h2>
<table>
<tr><th>Artikel</th><th>Specificatie</th><th>Aantal</th><th>Prijs/stuk</th><th>Bedrag</th></tr>
{bom_rows}
<tr class="totaal"><td>Totaal (excl. BTW)</td><td></td><td></td><td></td><td>{totaal}</td></tr>
</table>
<h2>Zaagplan</h2>
<table>
<tr><th>Profiel</th><th>Plank</th><th>Te zagen</th><th>Reststuk</th></tr>
{zaag_rows}
</table>
</body>
</html>"#,
            naam = html_escape(project_naam),
            datum = datum_vandaag(),
            zaagsnede = res.zaagsnede_mm,
            bom_rows = bom_rows,
            totaal = self.fmt_bedrag(res.totaal_excl),
            zaag_rows = zaag_rows
        )
    }

    fn afdrukken(&mut self, project_naam: &str, res: &Berekening) {
        let html = self.bom_html(project_naam, res);
        let path = std::env::temp_dir().join("speeltoestel-bom.html");
        match std::fs::write(&path, html) {
            Ok(()) => {
                let p = path.to_string_lossy().to_string();
                let spawned = if cfg!(target_os = "macos") {
                    std::process::Command::new("open").arg(&p).spawn()
                } else if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd").args(["/C", "start", "", &p]).spawn()
                } else {
                    std::process::Command::new("xdg-open").arg(&p).spawn()
                };
                self.status = match spawned {
                    Ok(_) => format!(
                        "BOM + zaagplan geopend in de browser — afdrukken met Cmd+P / Ctrl+P ({})",
                        path.display()
                    ),
                    Err(e) => format!(
                        "Browser openen mislukt: {e} — HTML staat klaar op {}",
                        path.display()
                    ),
                };
            }
            Err(e) => self.status = format!("HTML wegschrijven mislukt: {e}"),
        }
    }

    fn tab_calculatie(&mut self, ui: &mut egui::Ui) {
        let Some(project) = self.huidig_project() else {
            ui.heading("Calculatie");
            ui.label("Geen project geopend. Kies een project in het tabblad Projecten.");
            return;
        };
        let regels = project.regels.clone();
        let naam = project.naam.clone();
        ui.heading(format!("Calculatie: {naam}"));
        let res = bereken_project(&self.catalogus, &regels);
        ui.horizontal(|ui| {
            if primary_button(ui, "Afdrukken / exporteren…").clicked() {
                self.afdrukken(&naam, &res);
            }
        });
        ui.label(format!(
            "Zaagsnede: {} mm per zaagdeel — elk deel kost lengte + zaagsnede uit de ingekochte plank (een stuk van 1000 mm uit een plank van 2000 mm laat dus 997 mm rest over).",
            res.zaagsnede_mm
        ));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            card(ui, |ui| {
                ui.heading("Bill of Materials — aan te schaffen");
                egui::Grid::new("bom").striped(true).spacing([14.0, 10.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Artikel").strong());
                    ui.label(egui::RichText::new("Specificatie").strong());
                    ui.label(egui::RichText::new("Aantal").strong());
                    ui.label(egui::RichText::new("Prijs/stuk").strong());
                    ui.label(egui::RichText::new("Bedrag").strong());
                    ui.end_row();
                    for zr in &res.zaag_resultaten {
                        match &zr.resultaat {
                            Some(r) => {
                                for (lengte, prijs, aantal, bedrag) in &r.per_lengte {
                                    ui.label(zr.profiel.naam());
                                    ui.label(format!("{lengte} mm"));
                                    ui.label(aantal.to_string());
                                    ui.label(self.fmt_bedrag(*prijs));
                                    ui.label(self.fmt_bedrag(*bedrag));
                                    ui.end_row();
                                }
                            }
                            None => {
                                ui.label(zr.profiel.naam());
                                ui.colored_label(
                                    egui::Color32::RED,
                                    "geen passende inkooplengte",
                                );
                                ui.label("");
                                ui.label("");
                                ui.label("");
                                ui.end_row();
                            }
                        }
                    }
                    for l in &res.per_stuk {
                        ui.label(&l.naam);
                        if l.eenheid.is_empty() {
                            ui.label(format!("per stuk (nodig {})", l.aantal));
                            ui.label(l.inkoop_aantal.to_string());
                            ui.label(self.fmt_bedrag(l.prijs_per_stuk));
                        } else {
                            ui.label(format!(
                                "nodig {} stuks · per {} {}",
                                l.aantal, l.aantal_per_eenheid, l.eenheid
                            ));
                            ui.label(format!("{} {}", l.inkoop_aantal, l.eenheid));
                            ui.label(self.fmt_bedrag(l.prijs_per_eenheid));
                        }
                        ui.label(self.fmt_bedrag(l.kosten));
                        ui.end_row();
                    }
                    ui.label(egui::RichText::new("Totaal (excl. BTW)").strong());
                    ui.label("");
                    ui.label("");
                    ui.label("");
                    ui.label(egui::RichText::new(self.fmt_bedrag(res.totaal_excl)).strong().size(16.0).color(ACCENT));
                    ui.end_row();
                });
            });
            card(ui, |ui| {
                ui.heading("Zaagplan");
                ui.label(format!(
                    "Per ingekochte plank zie je welke lengtes je eruit zaagt (zaagsnede {} mm per zaagdeel zit al in de rest verrekend).",
                    res.zaagsnede_mm
                ));
                ui.add_space(4.0);
                let mut heeft_plan = false;
                egui::Grid::new("zaagplan").striped(true).spacing([14.0, 10.0]).show(ui, |ui| {
                    ui.label(egui::RichText::new("Profiel").strong());
                    ui.label(egui::RichText::new("Plank").strong());
                    ui.label(egui::RichText::new("Te zagen").strong());
                    ui.label(egui::RichText::new("Reststuk").strong());
                    ui.end_row();
                    for zr in &res.zaag_resultaten {
                        if let Some(r) = &zr.resultaat {
                            for (i, bak) in r.bakken.iter().enumerate() {
                                heeft_plan = true;
                                let stukken: Vec<String> =
                                    bak.stukken.iter().map(|s| format!("{s} mm")).collect();
                                ui.label(zr.profiel.naam());
                                ui.label(format!("{} ({} mm)", i + 1, bak.lengte));
                                ui.label(stukken.join(" + "));
                                ui.label(format!("{} mm", bak.rest));
                                ui.end_row();
                            }
                        }
                    }
                });
                if !heeft_plan {
                    ui.label("Geen zaagdelen in dit project.");
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.style_mut(|s| {
            s.spacing.slider_width = 150.0;
            s.spacing.interact_size = egui::vec2(150.0, 32.0);
            s.spacing.item_spacing = egui::vec2(12.0, 8.0);
            s.spacing.button_padding = egui::vec2(16.0, 6.0);
        });
        self.sidebar(ctx);
        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::default().fill(PANEL_BG).inner_margin(egui::Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                self.topbar(ui);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(PANEL_BG).inner_margin(egui::Margin::same(14.0)))
            .show(ctx, |ui| match self.tab {
                Tab::Projecten => self.tab_projecten(ui),
                Tab::Materialen => self.tab_materialen(ui),
                Tab::Project => self.tab_project(ui),
                Tab::Calculatie => self.tab_calculatie(ui),
            });
        if self.setup_nodig {
            self.setup_modal(ctx);
        }
        self.autosave();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.forceer_opslaan();
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 780.0])
            .with_min_inner_size([960.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Speeltoestel Calculator",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::light();
            visuals.panel_fill = PANEL_BG;
            visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
            visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
            visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0_f32, egui::Color32::BLACK);
            visuals.selection.bg_fill = ACCENT;
            visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
            visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
            visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
            visuals.widgets.active.rounding = egui::Rounding::same(8.0);
            visuals.widgets.open.rounding = egui::Rounding::same(8.0);
            cc.egui_ctx.set_visuals(visuals);
            let mut app = App::default();
            app.init_bij_opstarten();
            Ok(Box::new(app))
        }),
    )
}
