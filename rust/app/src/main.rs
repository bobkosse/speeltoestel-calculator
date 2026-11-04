use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui;
use eframe::egui::Widget;
use speeltoestel_core::model::{Data, InkoopOptie, PerStuk, Profiel, ProjectLijn};
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
    data: Data,
    huidig_bestand: Option<PathBuf>,
    tab: Tab,
    status: String,
    nieuw_per_stuk: (String, String, String),
    nieuw_profiel_inkoop: (String, String),
}

impl Default for App {
    fn default() -> Self {
        App {
            data: Data::default(),
            huidig_bestand: None,
            tab: Tab::Materialen,
            status: "Onopgeslagen project".to_string(),
            nieuw_per_stuk: (String::new(), String::new(), String::new()),
            nieuw_profiel_inkoop: (String::new(), String::new()),
        }
    }
}

impl App {
    fn fmt_bedrag(&self, x: f64) -> String {
        let munt = &self.data.instellingen.munt;
        let symbool = if munt == "EUR" { "€" } else { munt.as_str() };
        format!("{symbool} {:.2}", x)
    }

    fn parse_getal(s: &str) -> f64 {
        s.replace(',', ".").parse().unwrap_or(0.0)
    }

    fn set_status(&mut self) {
        self.status = match &self.huidig_bestand {
            Some(p) => format!("Bestand: {}", p.display()),
            None => "Onopgeslagen project".to_string(),
        };
    }

    fn bestand_openen(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON-bestand", &["json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(tekst) => match serde_json::from_str::<Data>(&tekst) {
                    Ok(data) => {
                        self.data = data;
                        self.huidig_bestand = Some(path);
                        self.set_status();
                    }
                    Err(e) => self.status = format!("Kon bestand niet lezen: {e}"),
                },
                Err(e) => self.status = format!("Kon bestand niet openen: {e}"),
            }
        }
    }

    fn bestand_opslaan(&mut self, als_anders: bool) {
        let json = match serde_json::to_string_pretty(&self.data) {
            Ok(j) => j,
            Err(e) => {
                self.status = format!("Serialisatiefout: {e}");
                return;
            }
        };
        if !als_anders && self.huidig_bestand.is_some() {
            let path = self.huidig_bestand.clone().unwrap();
            match std::fs::write(&path, json) {
                Ok(()) => self.status = format!("Opgeslagen: {}", path.display()),
                Err(e) => self.status = format!("Opslaan mislukt: {e}"),
            }
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON-bestand", &["json"])
            .set_file_name("speeltoestel-project.json")
            .save_file()
        {
            match std::fs::write(&path, json) {
                Ok(()) => {
                    self.huidig_bestand = Some(path);
                    self.set_status();
                }
                Err(e) => self.status = format!("Opslaan mislukt: {e}"),
            }
        }
    }

    fn nieuw_project(&mut self) {
        self.data = Data::default();
        self.huidig_bestand = None;
        self.set_status();
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Speeltoestel Calculator");
            ui.label(egui::RichText::new(&self.status).weak().small());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Opslaan als…").clicked() {
                    self.bestand_opslaan(true);
                }
                if ui.button("Opslaan").clicked() {
                    self.bestand_opslaan(false);
                }
                if ui.button("Openen…").clicked() {
                    self.bestand_openen();
                }
                if ui.button("Nieuw").clicked() {
                    self.nieuw_project();
                }
            });
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
                egui::DragValue::new(&mut self.data.instellingen.zaagsnede_mm)
                    .speed(0.5)
                    .range(0.0..=20.0)
                    .ui(ui);
                ui.label("BTW (%):");
                egui::DragValue::new(&mut self.data.instellingen.btw_pct)
                    .speed(0.1)
                    .range(0.0..=100.0)
                    .ui(ui);
                ui.label("Munt:");
                egui::TextEdit::singleline(&mut self.data.instellingen.munt)
                    .desired_width(60.0)
                    .ui(ui);
            });
            ui.separator();

            ui.heading("Producten per stuk");
            ui.label("Schroeven, bouten, etc. — prijs per stuk, exclusief BTW.");
            egui::Grid::new("per_stuk_grid").show(ui, |ui| {
                ui.label(egui::RichText::new("Naam").strong());
                ui.label(egui::RichText::new("Prijs/stuk").strong());
                ui.label(egui::RichText::new("Extra info").strong());
                ui.label("");
                ui.end_row();
                let mut verwijder: Option<String> = None;
                for m in self.data.per_stuk.iter_mut() {
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
                    self.data.per_stuk.retain(|m| m.id != id);
                    self.data.project.retain(|l| match l {
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
                        self.data.per_stuk.push(PerStuk {
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
            let profielen_len = self.data.profielen.len();
            for pi in 0..profielen_len {
                let profiel_id = self.data.profielen[pi].id.clone();
                egui::CollapsingHeader::new(format!("Profiel: {}", self.data.profielen[pi].naam()))
                    .id_salt(&profiel_id)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Houtsoort:");
                            egui::TextEdit::singleline(&mut self.data.profielen[pi].houtsoort)
                                .desired_width(140.0)
                                .ui(ui);
                            ui.label("Dikte (mm):");
                            egui::DragValue::new(&mut self.data.profielen[pi].dikte_mm)
                                .speed(1.0)
                                .range(1..=500)
                                .ui(ui);
                            ui.label("Breedte (mm):");
                            egui::DragValue::new(&mut self.data.profielen[pi].breedte_mm)
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
                            let inkoop_len = self.data.profielen[pi].inkoop.len();
                            for oi in 0..inkoop_len {
                                egui::DragValue::new(&mut self.data.profielen[pi].inkoop[oi].lengte_mm)
                                    .speed(10.0)
                                    .range(1..=100000)
                                    .ui(ui);
                                egui::DragValue::new(&mut self.data.profielen[pi].inkoop[oi].prijs)
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
                                    self.data.profielen[pi].inkoop.push(InkoopOptie {
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
                self.data.profielen.retain(|p| p.id != id);
                self.data.project.retain(|l| match l {
                    ProjectLijn::Zaagdeel { profiel_id, .. } => {
                        profiel_id.as_deref() != Some(id.as_str())
                    }
                    _ => true,
                });
            }
            if let Some((pid, oi)) = verwijder_inkoop {
                if let Some(p) = self.data.profielen.iter_mut().find(|p| p.id == pid) {
                    if oi < p.inkoop.len() {
                        p.inkoop.remove(oi);
                    }
                }
            }
            if ui.button("+ Profiel toevoegen").clicked() {
                self.data.profielen.push(Profiel {
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
                let len = self.data.project.len();
                for i in 0..len {
                    let lijn_id = self.data.project[i].id().to_string();
                    let is_zaagdeel = matches!(self.data.project[i], ProjectLijn::Zaagdeel { .. });
                    let mut nieuw_type_zaagdeel = is_zaagdeel;
                    egui::ComboBox::from_id_salt(format!("type_{lijn_id}"))
                        .selected_text(if is_zaagdeel { "Zaagdeel" } else { "Per stuk" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut nieuw_type_zaagdeel, true, "Zaagdeel");
                            ui.selectable_value(&mut nieuw_type_zaagdeel, false, "Per stuk");
                        });
                    if nieuw_type_zaagdeel != is_zaagdeel {
                        let id = lijn_id.clone();
                        if let Some(l) = self.data.project.iter_mut().find(|l| l.id() == id) {
                            *l = if nieuw_type_zaagdeel {
                                ProjectLijn::Zaagdeel {
                                    id,
                                    profiel_id: self.data.profielen.first().map(|p| p.id.clone()),
                                    lengte_mm: 0,
                                    aantal: 1,
                                }
                            } else {
                                ProjectLijn::PerStuk {
                                    id,
                                    materiaal_id: self.data.per_stuk.first().map(|m| m.id.clone()),
                                    aantal: 1,
                                }
                            };
                        }
                    }
                    match &mut self.data.project[i] {
                        ProjectLijn::PerStuk {
                            materiaal_id, aantal, ..
                        } => {
                            let mut gekozen = materiaal_id.clone().unwrap_or_default();
                            egui::ComboBox::from_id_salt(format!("mat_{lijn_id}"))
                                .selected_text(
                                    self.data
                                        .per_stuk
                                        .iter()
                                        .find(|m| Some(&m.id) == materiaal_id.as_ref())
                                        .map(|m| m.naam.clone())
                                        .unwrap_or_else(|| "– kies –".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    for m in &self.data.per_stuk {
                                        ui.selectable_value(&mut gekozen, m.id.clone(), &m.naam);
                                    }
                                });
                            if gekozen != materiaal_id.as_deref().unwrap_or("") || materiaal_id.is_none() && !gekozen.is_empty() {
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
                                    self.data
                                        .profielen
                                        .iter()
                                        .find(|p| Some(&p.id) == profiel_id.as_ref())
                                        .map(|p| p.naam())
                                        .unwrap_or_else(|| "– kies –".to_string()),
                                )
                                .show_ui(ui, |ui| {
                                    for p in &self.data.profielen {
                                        ui.selectable_value(&mut gekozen, p.id.clone(), p.naam());
                                    }
                                });
                            if gekozen != profiel_id.as_deref().unwrap_or("") || profiel_id.is_none() && !gekozen.is_empty() {
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
                    self.data.project.retain(|l| l.id() != id);
                }
            });
            ui.horizontal(|ui| {
                if ui.button("+ Per stuk toevoegen").clicked() {
                    self.data.project.push(ProjectLijn::PerStuk {
                        id: nieuw_id(),
                        materiaal_id: self.data.per_stuk.first().map(|m| m.id.clone()),
                        aantal: 1,
                    });
                }
                if ui.button("+ Zaagdeel toevoegen").clicked() {
                    self.data.project.push(ProjectLijn::Zaagdeel {
                        id: nieuw_id(),
                        profiel_id: self.data.profielen.first().map(|p| p.id.clone()),
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
            let res = bereken_project(&self.data);
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
                self.data.instellingen.btw_pct,
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
