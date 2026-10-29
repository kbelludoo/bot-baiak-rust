use bot_baiak_rust::game::catalog::{get_all_hunts, Catalog};

#[test]
fn test_baiak_hunts_catalog_coverage() {
    let hunts = get_all_hunts();
    assert!(hunts.len() >= 80);
    assert!(Catalog::has_route("refiner-cave"));
    assert!(Catalog::has_route("radiant-skyhold"));
    assert!(!Catalog::has_route("non-existent-cave"));
}

#[test]
fn test_baiak_route_solver() {
    let catalog = Catalog::default();
    let area_low = catalog.select_best_hunt(15);
    assert_eq!(area_low, "cyclopolis");

    let area_mid = catalog.select_best_hunt(130);
    assert_eq!(area_mid, "wyrm-cave");

    let area_high = catalog.select_best_hunt(1500);
    assert_eq!(area_high, "draklightsource-cave");
}

#[tokio::test]
async fn test_conta_2_sync_and_matchmake() {
    let content = std::fs::read_to_string("accounts.json").expect("ler accounts.json");
    let configs: Vec<bot_baiak_rust::game::types::AccountConfig> = serde_json::from_str(&content).expect("parse accounts.json");
    let acc2 = configs.iter().find(|c| c.id == "acc-2").expect("encontrar acc-2");
    assert_eq!(acc2.name, "Conta 2 - mister k");
    assert!(acc2.enabled);

    let trpc = bot_baiak_rust::game::trpc::TRPCClient::new(&acc2.credentials.token, &acc2.credentials.origin);
    if let Ok(chars) = trpc.get_characters().await {
        if !chars.is_empty() {
            println!("Personagem detectado na conta 2: {} (Level {})", chars[0].name, chars[0].level);
            assert_eq!(chars[0].name, "mister k");
            assert!(chars[0].level >= 200);
        }
    }
}
