use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use eframe::egui::Widget;
use speeltoestel_core::model::{Catalogus, InkoopOptie, PerStuk, Profiel, ProjectEntry, ProjectLijn, ProjectenData};
use speeltoestel_core::optimizer::bereken_project;

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
    bestand_bibliotheek: Option<PathBuf>,
    bestand_projecten: Option<PathBuf>,
    tab: Tab,
    mat_tab: MatTab,
    status: String,
    nieuw_per_stuk: (String, String, String),
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
            bestand_bibliotheek: None,
            bestand_projecten: None,
            tab: Tab::Projecten,
            mat_tab: MatTab::PerStuk,
            status: "Onopgeslagen wijzigingen".to_string(),
            nieuw_per_stuk: (String::new(), String::new(), String::new()),
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
        let bib = self
            .bestand_bibliotheek
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "onopgeslagen".to_string());
        let proj = self
            .bestand_projecten
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "onopgeslagen".to_string());
        self.status = format!("Bibliotheek: {bib}  ·  Projecten: {proj}");
    }

    fn huidig_project(&self) -> Option<&ProjectEntry> {
        self.huidig_project
            .as_ref()
            .and_then(|id| self.projecten.iter().find(|p| &p.id == id))
    }

    fn lees_projecten_bestand(path: &std::path::Path) -> Result<(Option<Catalogus>, ProjectenData), String> {
        let tekst = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let value: serde_json::Value =
            serde_json::from_str(&tekst).map_err(|e| format!("geen geldige JSON: {e}"))?;
        if value.get("projecten").is_some() {
            let data: ProjectenData = serde_json::from_value(value)
                .map_err(|e| format!("geen geldig projectenbestand: {e}"))?;
            Ok((None, data))
        } else if value.get("project").is_some() || value.get("per_stuk").is_some() || value.get("profielen").is_some() {
            // legacy: combined bestand (catalogus + project) of enkel project-array
            let catalogus = if value.get("per_stuk").is_some() || value.get("profielen").is_some() {
                Some(serde_json::from_value(value.clone()).map_err(|e| e.to_string())?)
            } else {
                None
            };
            let regels: Vec<ProjectLijn> = serde_json::from_value(value["project"].clone())
                .map_err(|e| format!("geen geldige projectregels: {e}"))?;
            Ok((
                catalogus,
                ProjectenData {
                    versie: 1,
                    projecten: vec![ProjectEntry {
                        id: nieuw_id(),
                        naam: "Geïmporteerd project".to_string(),
                        regels,
                    }],
                },
            ))
        } else {
            Err("geen projectenbestand (geen \"projecten\" of \"project\" gevonden)".to_string())
        }
    }

    fn projecten_openen(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON-bestand", &["json"])
            .pick_file()
        {
            match Self::lees_projecten_bestand(&path) {
                Ok((catalogus, data)) => {
                    if let Some(c) = catalogus {
                        self.catalogus = c;
                    }
                    self.projecten = data.projecten;
                    self.huidig_project = self.projecten.first().map(|p| p.id.clone());
                    self.bestand_projecten = Some(path);
                    self.set_status();
                }
                Err(e) => self.status = format!("Kon bestand niet lezen: {e}"),
            }
        }
    }

    fn projecten_opslaan(&mut self, als_anders: bool) {
        let data = ProjectenData {
            versie: 1,
            projecten: self.projecten.clone(),
        };
        let json = match serde_json::to_string_pretty(&data) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Serialisatiefout: {e}");
                return;
            }
        };
        let mut path = if als_anders { None } else { self.bestand_projecten.clone() };
        if path.is_none() {
            path = rfd::FileDialog::new()
                .add_filter("JSON-bestand", &["json"])
                .set_file_name("projecten.json")
                .save_file();
        }
        if let Some(p) = path {
            match std::fs::write(&p, json) {
                Ok(()) => {
                    self.bestand_projecten = Some(p);
                    self.set_status();
                }
                Err(e) => self.status = format!("Opslaan mislukt: {e}"),
            }
        }
    }

    fn bibliotheek_opslaan(&mut self, als_anders: bool) {
        let json = match serde_json::to_string_pretty(&self.catalogus) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Serialisatiefout: {e}");
                return;
            }
        };
        let mut path = if als_anders { None } else { self.bestand_bibliotheek.clone() };
        if path.is_none() {
            path = rfd::FileDialog::new()
                .add_filter("JSON-bestand", &["json"])
                .set_file_name("materialen.json")
                .save_file();
        }
        if let Some(p) = path {
            match std::fs::write(&p, json) {
                Ok(()) => {
                    self.bestand_bibliotheek = Some(p);
                    self.set_status();
                }
                Err(e) => self.status = format!("Opslaan mislukt: {e}"),
            }
        }
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
                if ui.button("Projecten opslaan als…").clicked() {
                    self.projecten_opslaan(true);
                }
                if ui.button("Projecten opslaan").clicked() {
                    self.projecten_opslaan(false);
                }
                if ui.button("Projecten openen…").clicked() {
                    self.projecten_openen();
                }
                ui.separator();
                if ui.button("Bibliotheek opslaan als…").clicked() {
                    self.bibliotheek_opslaan(true);
                }
                if ui.button("Bibliotheek opslaan").clicked() {
                    self.bibliotheek_opslaan(false);
                }
            });
        });
        ui.separator();
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
            ui.label("Schroeven, bouten, etc. — prijs per stuk, exclusief BTW. Deze bibliotheek deel je tussen alle projecten.");
            egui::Grid::new("per_stuk_grid").striped(true).spacing([14.0, 10.0]).show(ui, |ui| {
                ui.label(egui::RichText::new("Naam").strong());
                ui.label(egui::RichText::new("Prijs/stuk").strong());
                ui.label(egui::RichText::new("Extra info").strong());
                ui.label("");
                ui.end_row();
                let mut verwijder: Option<String> = None;
                for m in self.catalogus.per_stuk.iter_mut() {
                    text_field(ui, &mut m.naam, 280.0);
                    egui::DragValue::new(&mut m.prijs)
                        .speed(0.01)
                        .range(0.0..=100000.0)
                        .ui(ui);
                    text_field(ui, &mut m.info, 300.0);
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
            });
            ui.horizontal(|ui| {
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.0)
                    .desired_width(280.0)
                    .hint_text("Naam")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.1)
                    .desired_width(110.0)
                    .hint_text("Prijs/stuk")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.2)
                    .desired_width(280.0)
                    .hint_text("Extra info")
                    .ui(ui);
                if primary_button(ui, "Toevoegen").clicked() {
                    if !self.nieuw_per_stuk.0.trim().is_empty() {
                        let prijs = Self::parse_getal(&self.nieuw_per_stuk.1);
                        self.catalogus.per_stuk.push(PerStuk {
                            id: nieuw_id(),
                            naam: self.nieuw_per_stuk.0.trim().to_string(),
                            prijs,
                            info: self.nieuw_per_stuk.2.trim().to_string(),
                        });
                        self.nieuw_per_stuk = (String::new(), String::new(), String::new());
                    }
                }
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
                                ui.label(zr.profiel.naam());
                                ui.label(format!("{} mm", r.lengte_mm));
                                ui.label(r.aantal.to_string());
                                ui.label(self.fmt_bedrag(r.prijs_per_stuk));
                                ui.label(self.fmt_bedrag(zr.kosten));
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
                            }
                        }
                        ui.end_row();
                    }
                    for l in &res.per_stuk {
                        ui.label(&l.naam);
                        ui.label("per stuk");
                        ui.label(l.aantal.to_string());
                        ui.label(self.fmt_bedrag(l.prijs_per_stuk));
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
                ui.heading("Reststukken (ter controle)");
                let mut heeft_rest = false;
                for zr in &res.zaag_resultaten {
                    if let Some(r) = &zr.resultaat {
                        for (i, bak) in r.bakken.iter().enumerate() {
                            heeft_rest = true;
                            let stukken: Vec<String> =
                                bak.stukken.iter().map(|s| format!("{s} mm")).collect();
                            ui.label(format!(
                                "{} — plank {} ({} mm): {} → reststuk {} mm",
                                zr.profiel.naam(),
                                i + 1,
                                r.lengte_mm,
                                stukken.join(" + "),
                                bak.rest
                            ));
                        }
                    }
                }
                if !heeft_rest {
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
            Ok(Box::new(App::default()))
        }),
    )
}
