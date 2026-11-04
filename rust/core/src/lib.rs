pub mod model;
pub mod optimizer;

#[cfg(test)]
mod tests {
    use super::model::*;
    use super::optimizer::*;

    #[test]
    fn zaagsnede_1x1000_uit_2000_geeft_rest_997() {
        let bakken = pack_optimaal(&[1000], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 997);
    }

    #[test]
    fn zaagsnede_2x100_past_in_2000() {
        let bakken = pack_optimaal(&[100, 100], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 2000 - 100 - 3 - 100 - 3);
    }

    #[test]
    fn zaagsnede_2x1000_past_niet_in_2000() {
        let bakken = pack_optimaal(&[1000, 1000], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 2);
    }

    #[test]
    fn zaagsnede_3x500_past_in_2000() {
        let bakken = pack_optimaal(&[500, 500, 500], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 2000 - 1500 - 3 * 3);
    }

    #[test]
    fn te_lang_deel_kan_niet() {
        assert!(pack_optimaal(&[2500], 2000, 3).is_none());
    }

    #[test]
    fn goedkoopste_lengte_2x900() {
        let res = optimaliseer_profiel(&[900, 900], &[(1000, 10.0), (2000, 25.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(1000, 10.0, 2, 20.0)]);
        assert_eq!(res.aantal, 2);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn langere_plank_wint_als_delen_samen_passen() {
        let res = optimaliseer_profiel(&[900, 900], &[(1000, 12.0), (2000, 20.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(2000, 20.0, 1, 20.0)]);
        assert_eq!(res.aantal, 1);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn zaagsnede_telt_mee_bij_lengtekeuze() {
        // 1000 mm stuk past niet in 1000 mm plank (1003 > 1000), 2100 mm wel (2006 <= 2100)
        let res = optimaliseer_profiel(&[1000, 1000], &[(1000, 12.0), (2100, 22.0)], 3).unwrap();
        assert_eq!(res.per_lengte, vec![(2100, 22.0, 1, 22.0)]);
        assert_eq!(res.kosten, 22.0);
    }

    #[test]
    fn user_voorbeeld_400_en_2000_goedkoopst_gemengd() {
        // nodig: 400 mm + 2000 mm; 1 x 2500 mm kost 60, maar 2100 mm (2000+zaagsnede) + 500 mm (400+zaagsnede) = 52
        let res = optimaliseer_profiel(&[400, 2000], &[(2500, 60.0), (2100, 40.0), (500, 12.0)], 3).unwrap();
        assert_eq!(res.kosten, 52.0);
        let mut per_lengte = res.per_lengte.clone();
        per_lengte.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(per_lengte, vec![(500, 12.0, 1, 12.0), (2100, 40.0, 1, 40.0)]);
    }

    #[test]
    fn gemengde_lengtes_winnen_als_dat_goedkoper_is() {
        // [2100, 1000, 1000, 1000] met 2000@48.50 en 2500@55, zaagsnede 3:
        // 3 x 2500 = 165,00, maar 2 x 2500 (2100; 1000+1000) + 1 x 2000 (1000) = 158,50
        let res = optimaliseer_profiel(&[2100, 1000, 1000, 1000], &[(2000, 48.50), (2500, 55.0)], 3).unwrap();
        assert_eq!(res.kosten, 158.50);
        assert_eq!(res.aantal, 3);
        let mut per_lengte = res.per_lengte.clone();
        per_lengte.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(per_lengte, vec![(2000, 48.50, 1, 48.50), (2500, 55.0, 2, 110.0)]);
    }

    #[test]
    fn lege_inkoop_is_geen_resultaat() {
        assert!(optimaliseer_profiel(&[100], &[], 3).is_none());
    }

    fn test_catalogus() -> Catalogus {
        Catalogus {
            versie: 1,
            instellingen: Instellingen {
                zaagsnede_mm: 3.0,
                munt: "EUR".to_string(),
            },
            per_stuk: vec![PerStuk {
                id: "s1".into(),
                naam: "Schroef 5x60".into(),
                prijs: 0.50,
                info: String::new(),
                eenheid: String::new(),
                aantal_per_eenheid: 0.0,
                prijs_per_eenheid: 0.0,
            }],
            profielen: vec![Profiel {
                id: "p1".into(),
                houtsoort: "Robinia".into(),
                dikte_mm: 45,
                breedte_mm: 120,
                inkoop: vec![InkoopOptie {
                    lengte_mm: 2000,
                    prijs: 40.0,
                }],
            }],
        }
    }

    #[test]
    fn bereken_project_totaal() {
        let catalogus = test_catalogus();
        let project = vec![
            ProjectLijn::PerStuk {
                id: "l1".into(),
                materiaal_id: Some("s1".into()),
                aantal: 10,
            },
            ProjectLijn::Zaagdeel {
                id: "l2".into(),
                profiel_id: Some("p1".into()),
                lengte_mm: 1000,
                aantal: 2,
            },
        ];
        let res = bereken_project(&catalogus, &project);
        assert_eq!(res.zaag_resultaten[0].resultaat.as_ref().unwrap().aantal, 2);
        assert_eq!(res.zaag_resultaten[0].kosten, 80.0);
        assert_eq!(res.totaal_excl, 85.0);
    }

    #[test]
    fn per_stuk_met_verpakking() {
        // doos van 100 stuks à €12,00 → stukprijs €0,12; nodig 24 → 1 doos = €12,00
        let mut catalogus = test_catalogus();
        catalogus.per_stuk = vec![PerStuk {
            id: "s1".into(),
            naam: "Schroef 5x60 (doos)".into(),
            prijs: 0.0,
            info: String::new(),
            eenheid: "doos".into(),
            aantal_per_eenheid: 100.0,
            prijs_per_eenheid: 12.0,
        }];
        let project = vec![ProjectLijn::PerStuk {
            id: "l1".into(),
            materiaal_id: Some("s1".into()),
            aantal: 24,
        }];
        let res = bereken_project(&catalogus, &project);
        let l = &res.per_stuk[0];
        assert!((l.prijs_per_stuk - 0.12).abs() < 1e-9);
        assert_eq!(l.aantal, 24);
        assert_eq!(l.inkoop_aantal, 1);
        assert_eq!(l.kosten, 12.0);
        assert_eq!(res.totaal_excl, 12.0);
    }

    #[test]
    fn bereken_project_deel_past_nergens() {
        let mut catalogus = Catalogus::default();
        catalogus.per_stuk.clear();
        catalogus.profielen = vec![Profiel {
            id: "p1".into(),
            houtsoort: "Vuren".into(),
            dikte_mm: 20,
            breedte_mm: 100,
            inkoop: vec![InkoopOptie {
                lengte_mm: 1000,
                prijs: 5.0,
            }],
        }];
        let project = vec![ProjectLijn::Zaagdeel {
            id: "l1".into(),
            profiel_id: Some("p1".into()),
            lengte_mm: 1500,
            aantal: 1,
        }];
        let res = bereken_project(&catalogus, &project);
        assert!(res.zaag_resultaten[0].resultaat.is_none());
        assert_eq!(res.totaal_excl, 0.0);
    }

    #[test]
    fn json_roundtrip_catalogus_en_project() {
        let catalogus = Catalogus::default();
        let json = serde_json::to_string_pretty(&catalogus).unwrap();
        let terug: Catalogus = serde_json::from_str(&json).unwrap();
        assert_eq!(terug.per_stuk.len(), 2);
        assert_eq!(terug.profielen.len(), 1);

        let projecten = ProjectenData {
            versie: 1,
            projecten: vec![ProjectEntry {
                id: "pr1".into(),
                naam: "Speeltoren".into(),
                regels: vec![
                    ProjectLijn::Zaagdeel {
                        id: "l1".into(),
                        profiel_id: Some("p1".into()),
                        lengte_mm: 1000,
                        aantal: 2,
                    },
                    ProjectLijn::PerStuk {
                        id: "l2".into(),
                        materiaal_id: Some("s1".into()),
                        aantal: 24,
                    },
                ],
            }],
        };
        let pjson = serde_json::to_string_pretty(&projecten).unwrap();
        let pterug: ProjectenData = serde_json::from_str(&pjson).unwrap();
        assert_eq!(pterug.projecten.len(), 1);
        assert_eq!(pterug.projecten[0].regels.len(), 2);
        let res = bereken_project(&terug, &pterug.projecten[0].regels);
        assert!(res.totaal_excl > 0.0);
    }

    #[test]
    fn legacy_gecombineerd_bestand_wordt_gesplitst() {
        // Oude bestanden bevatten catalogus + project in één JSON.
        let legacy = r#"{
            "versie": 1,
            "instellingen": { "zaagsnede_mm": 3, "btw_pct": 21, "munt": "EUR" },
            "per_stuk": [{ "id": "s1", "naam": "Schroef", "prijs": 0.5, "info": "" }],
            "profielen": [{ "id": "p1", "houtsoort": "Robinia", "dikte_mm": 45, "breedte_mm": 120,
                            "inkoop": [{ "lengte_mm": 2000, "prijs": 40 }] }],
            "project": [{ "id": "l1", "type": "zaagdeel", "profiel_id": "p1", "lengte_mm": 1000, "aantal": 2 }]
        }"#;
        let value: serde_json::Value = serde_json::from_str(legacy).unwrap();
        let catalogus: Catalogus = serde_json::from_value(value.clone()).unwrap();
        // legacy "project"-array wordt in de app verpakt in één ProjectEntry
        let regels: Vec<ProjectLijn> =
            serde_json::from_value(value["project"].clone()).unwrap();
        assert_eq!(catalogus.profielen.len(), 1);
        assert_eq!(regels.len(), 1);
        let res = bereken_project(&catalogus, &regels);
        assert_eq!(res.zaag_resultaten[0].kosten, 80.0);
    }
}
