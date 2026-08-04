use serde_json::Value;

#[test]
fn overlay_starts_hidden_and_transparent_until_web_content_is_ready() {
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let overlay = config["app"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|window| window["label"] == "overlay")
        .unwrap();

    assert_eq!(overlay["visible"], false);
    assert_eq!(overlay["transparent"], true);
    assert_eq!(overlay["backgroundColor"], serde_json::json!([0, 0, 0, 0]));
    assert_eq!(overlay["decorations"], false);
}
