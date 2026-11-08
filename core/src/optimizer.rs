use crate::model::{Catalogus, Profiel, ProjectLijn};

const FFD_LIMIET: usize = 40;

/// Een benodigd zaagdeel: lengte × breedte in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stuk {
    pub lengte: i64,
    pub breedte: i64,
}

/// Een ingekochte plank met de delen die eruit gezaagd worden.
#[derive(Debug, Clone)]
pub struct Bak {
    /// Planklengte in mm.
    pub lengte: i64,
    pub stukken: Vec<Stuk>,
    /// Resterende lengte in de laatste strook.
    pub rest: i64,
    /// Resterende breedte van de plank.
    pub rest_breedte: i64,
}

// ---------- alleen lengte (fallback / legacy) ----------

fn past_in_bak_lengte(lengte: i64, rest: i64, zaagsnede: i64) -> bool {
    if lengte == rest {
        return true;
    }
    lengte + zaagsnede <= rest
}

fn verbruik_lengte(lengte: i64, rest: i64, zaagsnede: i64) -> i64 {
    if lengte == rest {
        lengte
    } else {
        lengte + zaagsnede
    }
}

pub fn pack_ffd(lengtes: &[i64], stock_lengte: i64, zaagsnede: i64) -> Vec<Bak> {
    let mut items: Vec<i64> = lengtes.to_vec();
    items.sort_by(|a, b| b.cmp(a));
    let mut bakken: Vec<Bak> = Vec::new();
    for item in items {
        let mut geplaatst = false;
        for bak in bakken.iter_mut() {
            if past_in_bak_lengte(item, bak.rest, zaagsnede) {
                let v = verbruik_lengte(item, bak.rest, zaagsnede);
                bak.stukken.push(Stuk { lengte: item, breedte: 0 });
                bak.rest -= v;
                geplaatst = true;
                break;
            }
        }
        if !geplaatst {
            let v = if item == stock_lengte { item } else { item + zaagsnede };
            bakken.push(Bak {
                lengte: stock_lengte,
                stukken: vec![Stuk { lengte: item, breedte: 0 }],
                rest: stock_lengte - v,
                rest_breedte: 0,
            });
        }
    }
    bakken
}

/// Optimaal (minimaal aantal bakken) op lengte; None als een deel te lang is.
pub fn pack_optimaal(lengtes: &[i64], stock_lengte: i64, zaagsnede: i64) -> Option<Vec<Bak>> {
    let items: Vec<i64> = {
        let mut v = lengtes.to_vec();
        v.sort_by(|a, b| b.cmp(a));
        v
    };
    if items.is_empty() {
        return Some(Vec::new());
    }
    if items.len() > FFD_LIMIET {
        return Some(pack_ffd(lengtes, stock_lengte, zaagsnede));
    }
    if items.iter().any(|&l| l > stock_lengte) {
        return None;
    }

    struct Staat {
        bakken: Vec<Bak>,
        beste: Option<Vec<Bak>>,
    }

    fn rec(staat: &mut Staat, items: &[i64], i: usize, stock_lengte: i64, zaagsnede: i64) {
        if let Some(beste) = &staat.beste {
            if staat.bakken.len() >= beste.len() {
                return;
            }
        }
        if i == items.len() {
            if staat.beste.as_ref().map_or(true, |b| staat.bakken.len() < b.len()) {
                staat.beste = Some(staat.bakken.clone());
            }
            return;
        }
        let mut geprobeerd: Vec<(i64, usize)> = Vec::new();
        for j in 0..staat.bakken.len() {
            let sleutel = (staat.bakken[j].rest, staat.bakken[j].stukken.len());
            if geprobeerd.contains(&sleutel) {
                continue;
            }
            if past_in_bak_lengte(items[i], staat.bakken[j].rest, zaagsnede) {
                geprobeerd.push(sleutel);
                let v = verbruik_lengte(items[i], staat.bakken[j].rest, zaagsnede);
                staat.bakken[j].stukken.push(Stuk { lengte: items[i], breedte: 0 });
                staat.bakken[j].rest -= v;
                rec(staat, items, i + 1, stock_lengte, zaagsnede);
                staat.bakken[j].rest += v;
                staat.bakken[j].stukken.pop();
            }
        }
        let v = if items[i] == stock_lengte { items[i] } else { items[i] + zaagsnede };
        staat.bakken.push(Bak {
            lengte: stock_lengte,
            stukken: vec![Stuk { lengte: items[i], breedte: 0 }],
            rest: stock_lengte - v,
            rest_breedte: 0,
        });
        rec(staat, items, i + 1, stock_lengte, zaagsnede);
        staat.bakken.pop();
    }

    let mut staat = Staat { bakken: Vec::new(), beste: None };
    rec(&mut staat, &items, 0, stock_lengte, zaagsnede);
    staat.beste
}

// ---------- 2D: stroken over de breedte, per strook in de lengte ----------

#[derive(Clone)]
struct Strip {
    /// Breedte van deze strook.
    sw: i64,
    /// Resterende lengte in deze strook.
    rl: i64,
}

#[derive(Clone)]
struct Bin {
    lengte: i64,
    /// Resterende breedte van de plank.
    rw: i64,
    strips: Vec<Strip>,
    stukken: Vec<Stuk>,
}

impl Bin {
    fn nieuw(lengte: i64, breedte: i64) -> Bin {
        Bin { lengte, rw: breedte, strips: Vec::new(), stukken: Vec::new() }
    }

    fn strip_pas(&self, stuk: Stuk, zaagsnede: i64) -> bool {
        match self.strips.last() {
            Some(s) => stuk.breedte <= s.sw && past_in_bak_lengte(stuk.lengte, s.rl, zaagsnede),
            None => false,
        }
    }

    fn plaats_in_strip(&mut self, stuk: Stuk, zaagsnede: i64) {
        let s = self.strips.last_mut().unwrap();
        let v = verbruik_lengte(stuk.lengte, s.rl, zaagsnede);
        s.rl -= v;
        self.stukken.push(stuk);
    }

    /// Nieuwe strook starten: eerste strook gebruikt de fabriekskant (geen kerf),
    /// elke volgende strook kost zaagsnede + breedte (net als n delen = n−1 zaagsnedes).
    fn strip_kosten(&self, stuk: Stuk, zaagsnede: i64) -> i64 {
        if self.strips.is_empty() {
            stuk.breedte
        } else {
            zaagsnede + stuk.breedte
        }
    }

    fn nieuwe_strip_pas(&self, stuk: Stuk, zaagsnede: i64) -> bool {
        self.strip_kosten(stuk, zaagsnede) <= self.rw && past_in_bak_lengte(stuk.lengte, self.lengte, zaagsnede)
    }

    fn start_nieuwe_strip(&mut self, stuk: Stuk, zaagsnede: i64) {
        self.rw -= self.strip_kosten(stuk, zaagsnede);
        self.strips.push(Strip { sw: stuk.breedte, rl: self.lengte });
        self.plaats_in_strip(stuk, zaagsnede);
    }

    fn bak(&self) -> Bak {
        let (rest, rest_breedte) = match self.strips.last() {
            Some(s) => (s.rl, self.rw),
            None => (self.lengte, self.rw),
        };
        Bak { lengte: self.lengte, stukken: self.stukken.clone(), rest, rest_breedte }
    }
}

#[derive(Debug, Clone)]
pub struct ProfielResultaat {
    /// (lengte_mm, prijs_per_stuk, aantal, bedrag) per ingekochte lengte
    pub per_lengte: Vec<(i64, f64, usize, f64)>,
    pub aantal: usize,
    pub bakken: Vec<Bak>,
    pub kosten: f64,
    pub afval: i64,
}

fn maak_resultaat(bakken: Vec<Bak>, inkoop: &[(i64, f64)]) -> ProfielResultaat {
    let prijs_per_lengte = |l: i64| inkoop.iter().find(|(sl, _)| *sl == l).map(|(_, p)| *p).unwrap_or(0.0);
    let mut per_lengte: Vec<(i64, f64, usize, f64)> = Vec::new();
    for bak in &bakken {
        if let Some(e) = per_lengte.iter_mut().find(|(l, _, _, _)| *l == bak.lengte) {
            e.2 += 1;
            e.3 = e.2 as f64 * e.1;
        } else {
            let p = prijs_per_lengte(bak.lengte);
            per_lengte.push((bak.lengte, p, 1, p));
        }
    }
    let kosten: f64 = per_lengte.iter().map(|(_, _, _, b)| *b).sum();
    let afval: i64 = bakken.iter().map(|b| b.rest * b.rest_breedte).sum();
    ProfielResultaat { per_lengte, aantal: bakken.len(), bakken, kosten, afval }
}

/// Kies de goedkoopste samenstelling van inkooplengtes (gemengd mogelijk) voor een set zaagdelen.
/// `items` = (lengte, breedte) per benodigd deel; `plank_breedte` = breedte van het profiel.
pub fn optimaliseer_profiel(
    items: &[Stuk],
    plank_breedte: i64,
    inkoop: &[(i64, f64)],
    zaagsnede: i64,
) -> Option<ProfielResultaat> {
    if inkoop.is_empty() || items.is_empty() {
        return None;
    }
    if items.iter().any(|s| s.breedte > plank_breedte) {
        return None;
    }
    if items.iter().any(|s| !inkoop.iter().any(|&(sl, _)| s.lengte <= sl)) {
        return None;
    }

    let mut sortering: Vec<Stuk> = items.to_vec();
    sortering.sort_by(|a, b| b.lengte.cmp(&a.lengte).then(b.breedte.cmp(&a.breedte)));

    if sortering.len() > FFD_LIMIET {
        // fallback voor grote aantallen: alleen op lengte (breedte past per definitie)
        let lengtes: Vec<i64> = sortering.iter().map(|s| s.lengte).collect();
        let mut beste: Option<ProfielResultaat> = None;
        for &(l, p) in inkoop {
            let Some(bakken) = pack_optimaal(&lengtes, l, zaagsnede) else { continue };
            let kosten = bakken.len() as f64 * p;
            let afval: i64 = bakken.iter().map(|b| b.rest * plank_breedte).sum();
            let is_beter = match &beste {
                None => true,
                Some(b) => kosten < b.kosten - 1e-9 || ((kosten - b.kosten).abs() <= 1e-9 && afval < b.afval),
            };
            if is_beter {
                beste = Some(ProfielResultaat {
                    per_lengte: vec![(l, p, bakken.len(), kosten)],
                    aantal: bakken.len(),
                    bakken,
                    kosten,
                    afval,
                });
            }
        }
        return beste;
    }

    const NODE_LIMIT: u64 = 400_000;

    fn rec(
        bins: &mut Vec<Bin>,
        items: &[Stuk],
        i: usize,
        inkoop: &[(i64, f64)],
        plank_breedte: i64,
        zaagsnede: i64,
        kosten: f64,
        best: &mut Option<(f64, Vec<Bin>)>,
        nodes: &mut u64,
    ) {
        *nodes += 1;
        if *nodes > NODE_LIMIT {
            return;
        }
        if let Some((bc, _)) = best {
            if kosten >= *bc - 1e-9 {
                return;
            }
        }
        if i == items.len() {
            if best.as_ref().map_or(true, |(bc, _)| kosten < *bc - 1e-9) {
                *best = Some((kosten, bins.clone()));
            }
            return;
        }
        let stuk = items[i];
        let mut geprobeerd: Vec<(i64, i64, i64)> = Vec::new();
        for j in 0..bins.len() {
            let sleutel = (
                bins[j].strips.last().map_or(-1, |s| s.rl),
                bins[j].strips.last().map_or(-1, |s| s.sw),
                bins[j].rw,
            );
            if geprobeerd.contains(&sleutel) {
                continue;
            }
            if bins[j].strip_pas(stuk, zaagsnede) {
                geprobeerd.push(sleutel);
                bins[j].plaats_in_strip(stuk, zaagsnede);
                rec(bins, items, i + 1, inkoop, plank_breedte, zaagsnede, kosten, best, nodes);
                let s = bins[j].strips.last_mut().unwrap();
                s.rl += verbruik_lengte(stuk.lengte, s.rl, zaagsnede);
                bins[j].stukken.pop();
            }
            if bins[j].nieuwe_strip_pas(stuk, zaagsnede) {
                let oude_rw = bins[j].rw;
                let oude_strips = bins[j].strips.len();
                bins[j].start_nieuwe_strip(stuk, zaagsnede);
                rec(bins, items, i + 1, inkoop, plank_breedte, zaagsnede, kosten, best, nodes);
                bins[j].strips.truncate(oude_strips);
                bins[j].rw = oude_rw;
                bins[j].stukken.pop();
            }
        }
        for &(l, p) in inkoop {
            if stuk.lengte <= l && stuk.breedte <= plank_breedte {
                bins.push(Bin::nieuw(l, plank_breedte));
                bins.last_mut().unwrap().start_nieuwe_strip(stuk, zaagsnede);
                rec(bins, items, i + 1, inkoop, plank_breedte, zaagsnede, kosten + p, best, nodes);
                bins.pop();
            }
        }
    }

    // greedy seed (FFD-achtig) zodat branch & bound direct sterk kan prunen
    let mut bins: Vec<Bin> = Vec::new();
    let mut kosten = 0.0;
    for stuk in &sortering {
        let mut geplaatst = false;
        for j in 0..bins.len() {
            if bins[j].strip_pas(*stuk, zaagsnede) {
                bins[j].plaats_in_strip(*stuk, zaagsnede);
                geplaatst = true;
                break;
            }
            if bins[j].nieuwe_strip_pas(*stuk, zaagsnede) {
                bins[j].start_nieuwe_strip(*stuk, zaagsnede);
                geplaatst = true;
                break;
            }
        }
        if !geplaatst {
            let mut beste_l = None;
            let mut beste_p = f64::MAX;
            for &(l, p) in inkoop {
                if stuk.lengte <= l && stuk.breedte <= plank_breedte && p < beste_p {
                    beste_l = Some(l);
                    beste_p = p;
                }
            }
            if let Some(l) = beste_l {
                bins.push(Bin::nieuw(l, plank_breedte));
                bins.last_mut().unwrap().start_nieuwe_strip(*stuk, zaagsnede);
                kosten += beste_p;
            }
        }
    }
    let mut best: Option<(f64, Vec<Bin>)> = if bins.is_empty() {
        None
    } else {
        Some((kosten, bins.clone()))
    };
    bins.clear();
    let mut nodes: u64 = 0;
    rec(&mut bins, &sortering, 0, inkoop, plank_breedte, zaagsnede, 0.0, &mut best, &mut nodes);
    best.map(|(_, bins)| {
        let bakken = bins.iter().map(|b| b.bak()).collect();
        maak_resultaat(bakken, inkoop)
    })
}

pub fn rond2(x: f64) -> f64 {
    let r = (x * 100.0).round() / 100.0;
    if r == 0.0 { 0.0 } else { r } // normaliseer -0.0
}

#[derive(Debug, Clone)]
pub struct ModuleKosten {
    pub naam: String,
    pub aantal: u32,
    pub inkoop: f64,
    pub marge: f64,
    pub uren: f64,
    /// aantal × modulelengte (mm)
    pub lengte_totaal: i64,
    /// hout-inkoopdeel (standalone) — wordt na globale optimalisatie proportioneel geschaald
    pub hout: f64,
}

#[derive(Debug, Clone)]
pub struct Berekening {
    pub zaagsnede_mm: i64,
    pub per_stuk: Vec<PerStukKosten>,
    pub zaag_resultaten: Vec<ZaagResultaat>,
    pub modules: Vec<ModuleKosten>,
    pub uur_tarief: f64,
    /// Inkoopprijs materiaal (hout + losse producten), excl. BTW.
    pub inkoop_totaal: f64,
    /// Totale marge over losse inkoopproducten.
    pub marge_totaal: f64,
    /// Marge over houtinkoop (vaste % uit instellingen).
    pub marge_hout: f64,
    pub uren_totaal: f64,
    /// Arbeidskosten = uur_tarief × uren.
    pub arbeid: f64,
    /// Verkoopprijs = inkoop + marge + arbeid.
    pub verkoopprijs: f64,
    /// Bruto winst = verkoopprijs − inkoop.
    pub bruto_winst: f64,
    /// Totale lengte van het project (alle modules, in mm).
    pub lengte_totaal: i64,
    /// alias voor inkoop_totaal (backward compat)
    pub totaal_excl: f64,
}

#[derive(Debug, Clone)]
pub struct PerStukKosten {
    pub naam: String,
    pub aantal: u32,
    pub eenheid: String,
    pub aantal_per_eenheid: f64,
    pub inkoop_aantal: u32,
    pub prijs_per_eenheid: f64,
    pub prijs_per_stuk: f64,
    pub marge_pct: f64,
    pub kosten: f64,
    pub marge: f64,
}

#[derive(Debug, Clone)]
pub struct ZaagResultaat {
    pub profiel: Profiel,
    pub stukken: Vec<Stuk>,
    pub resultaat: Option<ProfielResultaat>,
    pub kosten: f64,
}

fn bereken_regels(catalogus: &Catalogus, regels: &[ProjectLijn], zaagsnede: i64) -> (Vec<PerStukKosten>, Vec<ZaagResultaat>) {
    let mut per_stuk: Vec<PerStukKosten> = Vec::new();
    let mut zaag_groepen: Vec<(String, Vec<Stuk>)> = Vec::new();

    for lijn in regels {
        match lijn {
            ProjectLijn::PerStuk { materiaal_id, aantal, .. } => {
                if let Some(m) = materiaal_id
                    .as_ref()
                    .and_then(|id| catalogus.per_stuk.iter().find(|m| &m.id == id))
                {
                    let stukprijs = m.stukprijs();
                    let inkoop_aantal = if m.aantal_per_eenheid > 0.0 {
                        ((*aantal as f64) / m.aantal_per_eenheid).ceil() as u32
                    } else {
                        *aantal
                    };
                    let kosten = rond2(*aantal as f64 * stukprijs);
                    // samenvoegen per product: zelfde product → één regel met totaalaantal
                    if let Some(e) = per_stuk.iter_mut().find(|e| e.naam == m.naam) {
                        e.aantal += *aantal;
                        e.inkoop_aantal = if m.aantal_per_eenheid > 0.0 {
                            ((e.aantal as f64) / m.aantal_per_eenheid).ceil() as u32
                        } else {
                            e.aantal
                        };
                        e.kosten = rond2(e.aantal as f64 * stukprijs);
                        e.marge = rond2(e.kosten * m.marge_pct / 100.0);
                    } else {
                        per_stuk.push(PerStukKosten {
                            naam: m.naam.clone(),
                            aantal: *aantal,
                            eenheid: m.eenheid.clone(),
                            aantal_per_eenheid: m.aantal_per_eenheid,
                            inkoop_aantal,
                            prijs_per_eenheid: m.prijs_per_eenheid,
                            prijs_per_stuk: stukprijs,
                            marge_pct: m.marge_pct,
                            marge: rond2(kosten * m.marge_pct / 100.0),
                            kosten,
                        });
                    }
                }
            }
            ProjectLijn::Zaagdeel { profiel_id, lengte_mm, breedte_mm, aantal, .. } => {
                let Some(pid) = profiel_id else { continue };
                let Some(profiel) = catalogus.profielen.iter().find(|p| p.id == *pid) else { continue };
                let breedte = breedte_mm.unwrap_or(profiel.breedte_mm).min(profiel.breedte_mm).max(0);
                if let Some(groep) = zaag_groepen.iter_mut().find(|(id, _)| id == pid) {
                    for _ in 0..*aantal {
                        groep.1.push(Stuk { lengte: *lengte_mm, breedte });
                    }
                } else {
                    zaag_groepen.push((pid.clone(), vec![Stuk { lengte: *lengte_mm, breedte }; *aantal as usize]));
                }
            }
        }
    }

    let mut zaag_resultaten: Vec<ZaagResultaat> = Vec::new();
    for (pid, stukken) in zaag_groepen {
        let Some(profiel) = catalogus.profielen.iter().find(|p| p.id == pid).cloned() else {
            continue;
        };
        let inkoop: Vec<(i64, f64)> = profiel.inkoop.iter().map(|o| (o.lengte_mm, o.prijs)).collect();
        let resultaat = optimaliseer_profiel(&stukken, profiel.breedte_mm, &inkoop, zaagsnede);
        let kosten = rond2(resultaat.as_ref().map_or(0.0, |r| r.kosten));
        zaag_resultaten.push(ZaagResultaat { profiel, stukken, resultaat, kosten });
    }
    zaag_resultaten.sort_by(|a, b| {
        (a.profiel.dikte_mm, a.profiel.breedte_mm, a.profiel.houtsoort.clone())
            .cmp(&(b.profiel.dikte_mm, b.profiel.breedte_mm, b.profiel.houtsoort.clone()))
    });
    (per_stuk, zaag_resultaten)
}

/// Verkoopprijs van één halffabrikaat (module): inkoop + marges + arbeid.
pub fn module_verkoopprijs(catalogus: &Catalogus, m: &crate::model::Module) -> f64 {
    let zaagsnede = catalogus.instellingen.zaagsnede_mm.round() as i64;
    let (ps, zr) = bereken_regels(catalogus, &m.regels, zaagsnede);
    let inkoop = ps.iter().map(|l| l.kosten).sum::<f64>() + zr.iter().map(|z| z.kosten).sum::<f64>();
    let hout = zr.iter().map(|z| z.kosten).sum::<f64>();
    let marge = ps.iter().map(|l| l.marge).sum::<f64>() + hout * catalogus.instellingen.marge_hout_pct / 100.0;
    let uren = m.uren_bouwen + m.uren_plaatsen;
    rond2(inkoop + marge + catalogus.instellingen.uur_tarief * uren)
}

pub fn bereken_project(catalogus: &Catalogus, project: &crate::model::ProjectEntry) -> Berekening {
    let zaagsnede = catalogus.instellingen.zaagsnede_mm.round() as i64;
    let uur_tarief = catalogus.instellingen.uur_tarief;

    // modules uit het project flattenen + legacy regels
    let mut alle_regels = project.alle_regels();
    let mut modules: Vec<ModuleKosten> = Vec::new();
    for pm in &project.modules {
        let Some(m) = pm
            .module_id
            .as_ref()
            .and_then(|id| catalogus.modules.iter().find(|m| &m.id == id))
        else {
            continue;
        };
        for _ in 0..pm.aantal {
            alle_regels.extend(m.regels.iter().cloned());
        }
        let (ps, zr) = bereken_regels(catalogus, &m.regels, zaagsnede);
        let n = pm.aantal as f64;
        let inkoop_m = rond2((ps.iter().map(|l| l.kosten).sum::<f64>() + zr.iter().map(|z| z.kosten).sum::<f64>()) * n);
        let hout_m = rond2(zr.iter().map(|z| z.kosten).sum::<f64>() * n);
        let marge_m = rond2((ps.iter().map(|l| l.marge).sum::<f64>() + zr.iter().map(|z| z.kosten).sum::<f64>() * catalogus.instellingen.marge_hout_pct / 100.0) * n);
        let uren_m = n * (m.uren_bouwen + m.uren_plaatsen);
        let lengte_m = pm.aantal as i64 * m.lengte_mm;
        modules.push(ModuleKosten {
            naam: m.naam.clone(),
            aantal: pm.aantal,
            inkoop: inkoop_m,
            marge: marge_m,
            uren: uren_m,
            lengte_totaal: lengte_m,
            hout: hout_m,
        });
    }

    let (per_stuk, zaag_resultaten) = bereken_regels(catalogus, &alle_regels, zaagsnede);
    // houtkosten per module schalen naar de globale optimum, zodat de module-kolommen
    // (inkoop/marge) precies optellen tot het projecttotaal
    let global_wood = zaag_resultaten.iter().map(|z| z.kosten).sum::<f64>();
    let standalone_wood: f64 = modules.iter().map(|m| m.hout).sum();
    let wood_factor = if standalone_wood > 1e-9 { global_wood / standalone_wood } else { 1.0 };
    if (wood_factor - 1.0).abs() > 1e-9 {
        for m in modules.iter_mut() {
            m.inkoop = rond2(m.inkoop - m.hout + m.hout * wood_factor);
            m.marge = rond2(m.marge + (m.hout * wood_factor - m.hout) * catalogus.instellingen.marge_hout_pct / 100.0);
        }
    }
    let inkoop_totaal = rond2(per_stuk.iter().map(|l| l.kosten).sum::<f64>() + zaag_resultaten.iter().map(|z| z.kosten).sum::<f64>());
    let hout_inkoop = rond2(zaag_resultaten.iter().map(|z| z.kosten).sum::<f64>());
    let marge_hout = rond2(hout_inkoop * catalogus.instellingen.marge_hout_pct / 100.0);
    let marge_totaal = rond2(per_stuk.iter().map(|l| l.marge).sum::<f64>() + marge_hout);
    let uren_totaal = modules.iter().map(|m| m.uren).sum();
    let arbeid = rond2(uur_tarief * uren_totaal);
    let verkoopprijs = rond2(inkoop_totaal + marge_totaal + arbeid);
    let bruto_winst = rond2(verkoopprijs - inkoop_totaal);
    let lengte_totaal = modules.iter().map(|m| m.lengte_totaal).sum();

    Berekening {
        zaagsnede_mm: zaagsnede,
        per_stuk,
        zaag_resultaten,
        modules,
        uur_tarief,
        inkoop_totaal,
        marge_totaal,
        marge_hout,
        uren_totaal,
        arbeid,
        verkoopprijs,
        bruto_winst,
        lengte_totaal,
        totaal_excl: inkoop_totaal,
    }
}
