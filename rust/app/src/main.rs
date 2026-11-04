use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use eframe::egui::Widget;
use speeltoestel_core::model::{Catalogus, InkoopOptie, PerStuk, Profiel, ProjectData, ProjectLijn};
use speeltoestel_core::optimizer::bereken_project;

static ID_TELLER: AtomicU64 = AtomicU64::new(0);

fn nieuw_id() -> String {
    let n = ID_TELLER.fetch_add(1, Ordering::SeqCst);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("id-{nanos}-{n}")
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Materialen,
    Project,
    Calculatie,
}

struct App {
    catalogus: Catalogus,
    project: Vec<ProjectLijn>,
    bestand_bibliotheek: Option<PathBuf>,
    bestand_project: Option<PathBuf>,
    tab: Tab,
    status: String,
    nieuw_per_stuk: (String, String, String),
    nieuw_profiel_inkoop: (String, String),
}

impl Default for App {
    fn default() -> Self {
        App {
            catalogus: Catalogus::default(),
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
            bestand_bibliotheek: None,
            bestand_project: None,
            tab: Tab::Materialen,
            status: "Nieuwe bibliotheek, onopgeslagen project".to_string(),
            nieuw_per_stuk: (String::new(), String::new(), String::new()),
            nieuw_profiel_inkoop: (String::new(), String::new()),
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
            .bestand_project
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "onopgeslagen".to_string());
        self.status = format!("Bibliotheek: {bib}  |  Project: {proj}");
    }

    fn lees_bestand(path: &std::path::Path) -> Result<(Catalogus, Vec<ProjectLijn>), String> {
        let tekst = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let value: serde_json::Value =
            serde_json::from_str(&tekst).map_err(|e| format!("geen geldige JSON: {e}"))?;
        let is_legacy = value.get("per_stuk").is_some() || value.get("profielen").is_some();
        if is_legacy {
            let catalogus: Catalogus = serde_json::from_value(value.clone())
                .map_err(|e| format!("geen geldige materialenbibliotheek: {e}"))?;
            let project: ProjectData = serde_json::from_value(value)
                .map_err(|e| format!("geen geldig project: {e}"))?;
            Ok((catalogus, project.project))
        } else {
            let project: ProjectData = serde_json::from_value(value)
                .map_err(|e| format!("geen geldig projectbestand: {e}"))?;
            Ok((Catalogus::default(), project.project))
        }
    }

    fn bibliotheek_openen(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON-bestand", &["json"])
            .pick_file()
        {
            let legacy = value_is_legacy(&path);
            match Self::lees_bestand(&path) {
                Ok((catalogus, project)) => {
                    self.catalogus = catalogus;
                    if legacy {
                        self.project = project;
                    }
                    self.bestand_bibliotheek = Some(path);
                    self.set_status();
                }
                Err(e) => self.status = format!("Kon bestand niet lezen: {e}"),
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

    fn project_openen(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON-bestand", &["json"])
            .pick_file()
        {
            match Self::lees_bestand(&path) {
                Ok((catalogus, project)) => {
                    let legacy = value_is_legacy(&path);
                    if legacy {
                        self.catalogus = catalogus;
                    }
                    self.project = project;
                    self.bestand_project = Some(path);
                    self.set_status();
                }
                Err(e) => self.status = format!("Kon bestand niet lezen: {e}"),
            }
        }
    }

    fn project_opslaan(&mut self, als_anders: bool) {
        let data = ProjectData {
            versie: 1,
            project: self.project.clone(),
        };
        let json = match serde_json::to_string_pretty(&data) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Serialisatiefout: {e}");
                return;
            }
        };
        let mut path = if als_anders { None } else { self.bestand_project.clone() };
        if path.is_none() {
            path = rfd::FileDialog::new()
                .add_filter("JSON-bestand", &["json"])
                .set_file_name("speeltoestel-project.json")
                .save_file();
        }
        if let Some(p) = path {
            match std::fs::write(&p, json) {
                Ok(()) => {
                    self.bestand_project = Some(p);
                    self.set_status();
                }
                Err(e) => self.status = format!("Opslaan mislukt: {e}"),
            }
        }
    }

    fn nieuw_project(&mut self) {
        self.project = Vec::new();
        self.bestand_project = None;
        self.set_status();
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Speeltoestel Calculator");
            ui.label(egui::RichText::new(&self.status).weak().small());
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Materialenbibliotheek:").strong());
            if ui.button("Openen…").clicked() {
                self.bibliotheek_openen();
            }
            if ui.button("Opslaan").clicked() {
                self.bibliotheek_opslaan(false);
            }
            if ui.button("Opslaan als…").clicked() {
                self.bibliotheek_opslaan(true);
            }
            ui.separator();
            ui.label(egui::RichText::new("Project:").strong());
            if ui.button("Nieuw").clicked() {
                self.nieuw_project();
            }
            if ui.button("Openen…").clicked() {
                self.project_openen();
            }
            if ui.button("Opslaan").clicked() {
                self.project_opslaan(false);
            }
            if ui.button("Opslaan als…").clicked() {
                self.project_opslaan(true);
            }
        });
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Tab::Materialen, "Materialen"),
                (Tab::Project, "Project"),
                (Tab::Calculatie, "Calculatie"),
            ] {
                if ui.selectable_label(self.tab == tab, label).clicked() {
                    self.tab = tab;
                }
            }
        });
        ui.separator();
    }

    fn tab_materialen(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Instellingen");
            ui.horizontal(|ui| {
                ui.label("Zaagsnede (mm):");
                egui::DragValue::new(&mut self.catalogus.instellingen.zaagsnede_mm)
                    .speed(0.5)
                    .range(0.0..=20.0)
                    .ui(ui);
                ui.label("BTW (%):");
                egui::DragValue::new(&mut self.catalogus.instellingen.btw_pct)
                    .speed(0.1)
                    .range(0.0..=100.0)
                    .ui(ui);
                ui.label("Munt:");
                egui::TextEdit::singleline(&mut self.catalogus.instellingen.munt)
                    .desired_width(60.0)
                    .ui(ui);
            });
            ui.separator();

            ui.heading("Producten per stuk");
            ui.label("Schroeven, bouten, etc. — prijs per stuk, exclusief BTW. Deze bibliotheek deel je tussen alle projecten.");
            egui::Grid::new("per_stuk_grid").show(ui, |ui| {
                ui.label(egui::RichText::new("Naam").strong());
                ui.label(egui::RichText::new("Prijs/stuk").strong());
                ui.label(egui::RichText::new("Extra info").strong());
                ui.label("");
                ui.end_row();
                let mut verwijder: Option<String> = None;
                for m in self.catalogus.per_stuk.iter_mut() {
                    egui::TextEdit::singleline(&mut m.naam)
                        .desired_width(200.0)
                        .ui(ui);
                    egui::DragValue::new(&mut m.prijs)
                        .speed(0.01)
                        .range(0.0..=100000.0)
                        .ui(ui);
                    egui::TextEdit::singleline(&mut m.info)
                        .desired_width(220.0)
                        .ui(ui);
                    if ui.button("Verwijder").clicked() {
                        verwijder = Some(m.id.clone());
                    }
                    ui.end_row();
                }
                if let Some(id) = verwijder {
                    self.catalogus.per_stuk.retain(|m| m.id != id);
                    self.project.retain(|l| match l {
                        ProjectLijn::PerStuk { materiaal_id, .. } => {
                            materiaal_id.as_deref() != Some(id.as_str())
                        }
                        _ => true,
                    });
                }
            });
            ui.horizontal(|ui| {
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.0)
                    .desired_width(200.0)
                    .hint_text("Naam")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.1)
                    .desired_width(90.0)
                    .hint_text("Prijs/stuk")
                    .ui(ui);
                egui::TextEdit::singleline(&mut self.nieuw_per_stuk.2)
                    .desired_width(200.0)
                    .hint_text("Extra info")
                    .ui(ui);
                if ui.button("Toevoegen").clicked() {
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
            ui.separator();

            ui.heading("Half-fabricaten (zaagdelen)");
            ui.label("Planken, palen, etc. Per profiel meerdere inkooplengtes mogelijk; de app kiest de goedkoopste samenstelling.");
            let mut verwijder_profiel: Option<String> = None;
            let mut verwijder_inkoop: Option<(String, usize)> = None;
            let profielen_len = self.catalogus.profielen.len();
            for pi in 0..profielen_len {
                let profiel_id = self.catalogus.profielen[pi].id.clone();
                egui::CollapsingHeader::new(format!("Profiel: {}", self.catalogus.profielen[pi].naam()))
                    .id_salt(&profiel_id)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Houtsoort:");
                            egui::TextEdit::singleline(&mut self.catalogus.profielen[pi].houtsoort)
                                .desired_width(140.0)
                                .ui(ui);
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
                            if ui.button("Profiel verwijderen").clicked() {
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
                                if ui.button("Verwijder").clicked() {
                                    verwijder_inkoop = Some((profiel_id.clone(), oi));
                                }
                                ui.end_row();
                            }
                        });
                        ui.horizontal(|ui| {
                            egui::TextEdit::singleline(&mut self.nieuw_profiel_inkoop.0)
                                .desired_width(110.0)
                                .hint_text("Lengte (mm)")
                                .ui(ui);
                            egui::TextEdit::singleline(&mut self.nieuw_profiel_inkoop.1)
                                .desired_width(100.0)
                                .hint_text("Prijs")
                                .ui(ui);
                            if ui.button("Inkooplengte toevoegen").clicked() {
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
                self.project.retain(|l| match l {
                    ProjectLijn::Zaagdeel { profiel_id, .. } => {
                        profiel_id.as_deref() != Some(id.as_str())
                    }
                    _ => true,
                });
            }
            if let Some((pid, oi)) = verwijder_inkoop {
                if let Some(p) = self.catalogus.profielen.iter_mut().find(|p| p.id == pid) {
                    if oi < p.inkoop.len() {
                        p.inkoop.remove(oi);
                    }
                }
            }
            if ui.button("+ Profiel toevoegen").clicked() {
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

    fn tab_project(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Benodigde onderdelen");
            egui::Grid::new("project_grid").show(ui, |ui| {
                ui.label(egui::RichText::new("Type").strong());
                ui.label(egui::RichText::new("Materiaal / profiel").strong());
                ui.label(egui::RichText::new("Lengte (mm)").strong());
                ui.label(egui::RichText::new("Aantal").strong());
                ui.label("");
                ui.end_row();
                let mut verwijder: Option<String> = None;
                let len = self.project.len();
                for i in 0..len {
                    let lijn_id = self.project[i].id().to_string();
                    let is_zaagdeel = matches!(self.project[i], ProjectLijn::Zaagdeel { .. });
                    let mut nieuw_type_zaagdeel = is_zaagdeel;
                    egui::ComboBox::from_id_salt(format!("type_{lijn_id}"))
                        .selected_text(if is_zaagdeel { "Zaagdeel" } else { "Per stuk" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut nieuw_type_zaagdeel, true, "Zaagdeel");
                            ui.selectable_value(&mut nieuw_type_zaagdeel, false, "Per stuk");
                        });
                    if nieuw_type_zaagdeel != is_zaagdeel {
                        let id = lijn_id.clone();
                        if let Some(l) = self.project.iter_mut().find(|l| l.id() == id) {
                            *l = if nieuw_type_zaagdeel {
                                ProjectLijn::Zaagdeel {
                                    id,
                                    profiel_id: self.catalogus.profielen.first().map(|p| p.id.clone()),
                                    lengte_mm: 0,
                                    aantal: 1,
                                }
                            } else {
                                ProjectLijn::PerStuk {
                                    id,
                                    materiaal_id: self.catalogus.per_stuk.first().map(|m| m.id.clone()),
                                    aantal: 1,
                                }
                            };
                        }
                    }
                    match &mut self.project[i] {
                        ProjectLijn::PerStuk {
                            materiaal_id, aantal, ..
                        } => {
                            let mut gekozen = materiaal_id.clone().unwrap_or_default();
                            egui::ComboBox::from_id_salt(format!("mat_{lijn_id}"))
                                .selected_text(
                                    self.catalogus
                                        .per_stuk
                                        .iter()
                                        .find(|m| Some(&m.id) == materiaal_id.as_ref())
                                        .map(|m| m.naam.clone())
                                        .unwrap_or_else(|| "– kies –".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    for m in &self.catalogus.per_stuk {
                                        ui.selectable_value(&mut gekozen, m.id.clone(), &m.naam);
                                    }
                                });
                            if gekozen != materiaal_id.as_deref().unwrap_or("") || (materiaal_id.is_none() && !gekozen.is_empty()) {
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
                            egui::ComboBox::from_id_salt(format!("prof_{lijn_id}"))
                                .selected_text(
                                    self.catalogus
                                        .profielen
                                        .iter()
                                        .find(|p| Some(&p.id) == profiel_id.as_ref())
                                        .map(|p| p.naam())
                                        .unwrap_or_else(|| "– kies –".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    for p in &self.catalogus.profielen {
                                        ui.selectable_value(&mut gekozen, p.id.clone(), p.naam());
                                    }
                                });
                            if gekozen != profiel_id.as_deref().unwrap_or("") || (profiel_id.is_none() && !gekozen.is_empty()) {
                                *profiel_id = Some(gekozen);
                            }
                            egui::DragValue::new(lengte_mm)
                                .speed(10.0)
                                .range(0..=100000)
                                .ui(ui);
                            egui::DragValue::new(aantal).speed(1.0).range(0..=100000).ui(ui);
                        }
                    }
                    if ui.button("Verwijder").clicked() {
                        verwijder = Some(lijn_id);
                    }
                    ui.end_row();
                }
                if let Some(id) = verwijder {
                    self.project.retain(|l| l.id() != id);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("+ Per stuk toevoegen").clicked() {
                    self.project.push(ProjectLijn::PerStuk {
                        id: nieuw_id(),
                        materiaal_id: self.catalogus.per_stuk.first().map(|m| m.id.clone()),
                        aantal: 1,
                    });
                }
                if ui.button("+ Zaagdeel toevoegen").clicked() {
                    self.project.push(ProjectLijn::Zaagdeel {
                        id: nieuw_id(),
                        profiel_id: self.catalogus.profielen.first().map(|p| p.id.clone()),
                        lengte_mm: 0,
                        aantal: 1,
                    });
                }
            });
        });
    }

    fn tab_calculatie(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Calculatie");
            let res = bereken_project(&self.catalogus, &self.project);
            ui.label(format!(
                "Zaagsnede per snede: {} mm — bij n delen uit één plank zijn er n−1 zaagsnedes nodig.",
                res.zaagsnede_mm
            ));
            ui.separator();
            if !res.zaag_resultaten.is_empty() {
                ui.heading("Zaagdelen per profiel");
                for zr in &res.zaag_resultaten {
                    ui.label(egui::RichText::new(zr.profiel.naam()).strong());
                    match &zr.resultaat {
                        None => {
                            ui.colored_label(
                                egui::Color32::RED,
                                "Geen inkooplengte gevonden waarin alle delen passen",
                            );
                        }
                        Some(r) => {
                            ui.label(format!(
                                "Inkoop: {} × {} mm à {} = {} (restafval totaal: {} mm)",
                                r.aantal,
                                r.lengte_mm,
                                self.fmt_bedrag(r.prijs_per_stuk),
                                self.fmt_bedrag(zr.kosten),
                                r.afval
                            ));
                            for (i, bak) in r.bakken.iter().enumerate() {
                                let stukken: Vec<String> =
                                    bak.stukken.iter().map(|s| format!("{s} mm")).collect();
                                ui.label(format!(
                                    "  Plank {} ({} mm): {} (zaagsnedes: {} × {} mm) → rest {} mm",
                                    i + 1,
                                    r.lengte_mm,
                                    stukken.join(" + "),
                                    bak.stukken.len().saturating_sub(1),
                                    res.zaagsnede_mm,
                                    bak.rest
                                ));
                            }
                        }
                    }
                    ui.separator();
                }
            }
            if !res.per_stuk.is_empty() {
                ui.heading("Producten per stuk");
                egui::Grid::new("calc_per_stuk").show(ui, |ui| {
                    ui.label(egui::RichText::new("Naam").strong());
                    ui.label(egui::RichText::new("Aantal").strong());
                    ui.label(egui::RichText::new("Prijs/stuk").strong());
                    ui.label(egui::RichText::new("Kosten").strong());
                    ui.end_row();
                    for l in &res.per_stuk {
                        ui.label(&l.naam);
                        ui.label(l.aantal.to_string());
                        ui.label(self.fmt_bedrag(l.prijs_per_stuk));
                        ui.label(self.fmt_bedrag(l.kosten));
                        ui.end_row();
                    }
                });
                ui.separator();
            }
            ui.label(egui::RichText::new(format!("Totaal exclusief BTW: {}", self.fmt_bedrag(res.totaal_excl))).strong());
            ui.label(format!(
                "BTW ({:.1}%): {}",
                self.catalogus.instellingen.btw_pct,
                self.fmt_bedrag(res.btw)
            ));
            ui.label(
                egui::RichText::new(format!("Totaal inclusief BTW: {}", self.fmt_bedrag(res.totaal_incl)))
                    .strong()
                    .size(18.0),
            );
        });
    }
}

fn value_is_legacy(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .map(|v| v.get("per_stuk").is_some() || v.get("profielen").is_some())
        .unwrap_or(false)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            self.header(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Materialen => self.tab_materialen(ui),
            Tab::Project => self.tab_project(ui),
            Tab::Calculatie => self.tab_calculatie(ui),
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Speeltoestel Calculator",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
