pub mod model;
pub mod optimizer;

#[cfg(test)]
mod tests {
    use super::model::*;
    use super::optimizer::*;

    #[test]
    fn zaagsnede_2x100_past_in_2000() {
        let bakken = pack_optimaal(&[100, 100], 2000, 3).unwrap();
        assert_eq!(bakken.len(), 1);
        assert_eq!(bakken[0].rest, 2000 - 100 - 3 - 100);
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
        assert_eq!(bakken[0].rest, 2000 - 1500 - 2 * 3);
    }

    #[test]
    fn te_lang_deel_kan_niet() {
        assert!(pack_optimaal(&[2500], 2000, 3).is_none());
    }

    #[test]
    fn goedkoopste_lengte_2x900() {
        let res = optimaliseer_profiel(&[900, 900], &[(1000, 10.0), (2000, 25.0)], 3).unwrap();
        assert_eq!(res.lengte_mm, 1000);
        assert_eq!(res.aantal, 2);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn langere_plank_wint_als_delen_samen_passen() {
        let res = optimaliseer_profiel(&[900, 900], &[(1000, 12.0), (2000, 20.0)], 3).unwrap();
        assert_eq!(res.lengte_mm, 2000);
        assert_eq!(res.aantal, 1);
        assert_eq!(res.kosten, 20.0);
    }

    #[test]
    fn zaagsnede_telt_mee_bij_lengtekeuze() {
        let res = optimaliseer_profiel(&[1000, 1000], &[(1000, 12.0), (2000, 20.0)], 3).unwrap();
        assert_eq!(res.lengte_mm, 1000);
        assert_eq!(res.aantal, 2);
        assert_eq!(res.kosten, 24.0);
    }

    #[test]
    fn lege_inkoop_is_geen_resultaat() {
        assert!(optimaliseer_profiel(&[100], &[], 3).is_none());
    }

    #[test]
    fn bereken_project_totaal() {
        let data = Data {
            versie: 1,
            instellingen: Instellingen {
                zaagsnede_mm: 3.0,
                btw_pct: 21.0,
                munt: "EUR".to_string(),
            },
            per_stuk: vec![PerStuk {
                id: "s1".into(),
                naam: "Schroef 5x60".into(),
                prijs: 0.50,
                info: String::new(),
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
            project: vec![
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
            ],
        };
        let res = bereken_project(&data);
        assert_eq!(res.zaag_resultaten[0].resultaat.as_ref().unwrap().aantal, 2);
        assert_eq!(res.zaag_resultaten[0].kosten, 80.0);
        assert_eq!(res.totaal_excl, 85.0);
        assert_eq!(res.btw, 17.85);
        assert_eq!(res.totaal_incl, 102.85);
    }

    #[test]
    fn bereken_project_deel_past_nergens() {
        let data = Data {
            versie: 1,
            instellingen: Instellingen {
                zaagsnede_mm: 3.0,
                btw_pct: 21.0,
                munt: "EUR".to_string(),
            },
            per_stuk: vec![],
            profielen: vec![Profiel {
                id: "p1".into(),
                houtsoort: "Vuren".into(),
                dikte_mm: 20,
                breedte_mm: 100,
                inkoop: vec![InkoopOptie {
                    lengte_mm: 1000,
                    prijs: 5.0,
                }],
            }],
            project: vec![ProjectLijn::Zaagdeel {
                id: "l1".into(),
                profiel_id: Some("p1".into()),
                lengte_mm: 1500,
                aantal: 1,
            }],
        };
        let res = bereken_project(&data);
        assert!(res.zaag_resultaten[0].resultaat.is_none());
        assert_eq!(res.totaal_excl, 0.0);
    }

    #[test]
    fn json_roundtrip_van_standaard_data() {
        let data = Data::default();
        let json = serde_json::to_string_pretty(&data).unwrap();
        let terug: Data = serde_json::from_str(&json).unwrap();
        assert_eq!(terug.per_stuk.len(), 2);
        assert_eq!(terug.profielen.len(), 1);
        assert_eq!(terug.project.len(), 2);
        let res = bereken_project(&terug);
        assert!(res.totaal_excl > 0.0);
    }
}
