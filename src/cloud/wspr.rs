#[derive(Debug, Clone, serde::Deserialize)]
pub struct WsprSpot {
    pub callsign: String,
    pub frequency: f64,
    pub snr: i32,
    pub gridsquare: String,
}

pub async fn fetch_wspr_spots(my_callsign: &str) -> Result<Vec<WsprSpot>, String> {
    let url = format!(
        "https://db1.wspr.live/?query=SELECT+callsign,frequency,snr,drift,gridsquare+FROM+wspr.rx+WHERE+rx_sign%3D%27{}%27+ORDER+BY+time+DESC+LIMIT+50+FORMAT+JSONEachRow",
        my_callsign
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
    
    if !resp.status().is_success() {
        return Err(format!("HTTP Error: {}", resp.status()));
    }
    
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let mut spots = Vec::new();
    for line in body.lines() {
        if let Ok(spot) = serde_json::from_str::<WsprSpot>(line) {
            spots.push(spot);
        }
    }
    Ok(spots)
}
