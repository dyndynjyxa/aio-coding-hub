pub(super) fn mask_header_value(name: &str, value: &str) -> serde_json::Value {
    let name_lc = name.trim().to_lowercase();
    if name_lc == "x-api-key" || name_lc == "authorization" {
        return serde_json::Value::String("***".to_string());
    }
    serde_json::Value::String(value.to_string())
}

pub(super) fn mask_response_header_value(name: &str, value: &str) -> serde_json::Value {
    let name_lc = name.trim().to_lowercase();
    if name_lc == "set-cookie"
        || name_lc == "cookie"
        || name_lc == "authorization"
        || name_lc == "proxy-authorization"
        || name_lc == "x-api-key"
    {
        return serde_json::Value::String("***".to_string());
    }
    serde_json::Value::String(value.to_string())
}
